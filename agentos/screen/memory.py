"""Visual memory — stores screenshots with CLIP embeddings for similarity search.

Uses open_clip ViT-B-32 to generate 512-dim embeddings, stored as BLOBs in
SQLite. Supports image-to-image and text-to-image similarity search via cosine
similarity on normalized vectors.
"""

from __future__ import annotations

import asyncio
import json
import uuid
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

import aiosqlite
import numpy as np

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.types import Screenshot

logger = get_logger("screen.memory")


@dataclass
class VisualMemoryEntry:
    """A single visual memory record."""

    id: str
    screenshot_hash: str
    embedding: list[float]  # 512 dims
    description: str
    context: str
    actions_taken: list[str]
    timestamp: datetime
    pinned: bool = False


@dataclass(frozen=True)
class MemorySearchResult:
    """Search result with similarity score."""

    entry: VisualMemoryEntry
    similarity: float  # 0.0-1.0


class VisualMemory:
    """Stores screenshots with CLIP embeddings in SQLite for similarity search."""

    def __init__(
        self,
        db_path: str | Path = "data/visual_memory.db",
        model_name: str = "ViT-B-32",
        max_entries: int = 1000,
    ) -> None:
        self._db_path = str(db_path)
        self._model_name = model_name
        self._max_entries = max_entries
        self._db: aiosqlite.Connection | None = None
        self._model = None
        self._preprocess = None
        self._tokenizer = None

    # ── lifecycle ────────────────────────────────────────────────

    async def initialize(self) -> None:
        """Create DB table. Model loads lazily on first embedding call."""
        if self._db_path != ":memory:":
            Path(self._db_path).parent.mkdir(parents=True, exist_ok=True)
        self._db = await aiosqlite.connect(self._db_path)
        await self._db.execute("PRAGMA journal_mode=WAL")
        await self._create_table()

    async def _create_table(self) -> None:
        assert self._db is not None
        await self._db.execute("""
            CREATE TABLE IF NOT EXISTS visual_memory (
                id TEXT PRIMARY KEY,
                screenshot_hash TEXT NOT NULL,
                embedding BLOB NOT NULL,
                description TEXT NOT NULL,
                context TEXT DEFAULT '',
                actions_taken TEXT DEFAULT '[]',
                pinned INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            )
        """)
        await self._db.execute(
            "CREATE INDEX IF NOT EXISTS idx_vm_hash ON visual_memory(screenshot_hash)"
        )
        await self._db.execute(
            "CREATE INDEX IF NOT EXISTS idx_vm_created ON visual_memory(created_at)"
        )
        await self._db.execute("CREATE INDEX IF NOT EXISTS idx_vm_pinned ON visual_memory(pinned)")
        await self._db.commit()

    async def close(self) -> None:
        """Close the database connection."""
        if self._db:
            await self._db.close()
            self._db = None

    # ── CLIP model ──────────────────────────────────────────────

    async def _load_model(self) -> None:
        """Load CLIP model lazily. Runs in thread to avoid blocking."""
        if self._model is not None:
            return

        def _load():  # type: ignore[no-untyped-def]
            import open_clip

            model, _, preprocess = open_clip.create_model_and_transforms(
                self._model_name, pretrained="laion2b_s34b_b79k"
            )
            tokenizer = open_clip.get_tokenizer(self._model_name)
            model.eval()
            return model, preprocess, tokenizer

        self._model, self._preprocess, self._tokenizer = await asyncio.to_thread(_load)
        logger.info("CLIP model %s loaded", self._model_name)

    def is_model_loaded(self) -> bool:
        """Check whether the CLIP model has been loaded."""
        return self._model is not None

    async def generate_embedding(self, image_bytes: bytes) -> list[float]:
        """Generate CLIP embedding for an image. Returns normalized 512-dim vector."""
        await self._load_model()

        import io

        import torch
        from PIL import Image

        def _embed() -> list[float]:
            img = Image.open(io.BytesIO(image_bytes)).convert("RGB")
            tensor = self._preprocess(img).unsqueeze(0)
            with torch.no_grad():
                features = self._model.encode_image(tensor)
                features /= features.norm(dim=-1, keepdim=True)
            return features[0].cpu().numpy().tolist()

        return await asyncio.to_thread(_embed)

    async def generate_text_embedding(self, text: str) -> list[float]:
        """Generate CLIP text embedding. Returns normalized 512-dim vector."""
        await self._load_model()

        import torch

        def _embed() -> list[float]:
            tokens = self._tokenizer([text])
            with torch.no_grad():
                features = self._model.encode_text(tokens)
                features /= features.norm(dim=-1, keepdim=True)
            return features[0].cpu().numpy().tolist()

        return await asyncio.to_thread(_embed)

    # ── storage ─────────────────────────────────────────────────

    async def store(
        self,
        screenshot: Screenshot,
        description: str,
        context: str = "",
        actions: list[str] | None = None,
    ) -> str:
        """Store a screenshot with its CLIP embedding. Returns the entry ID."""
        assert self._db is not None
        embedding = await self.generate_embedding(screenshot.image_bytes)
        entry_id = uuid.uuid4().hex[:12]
        embedding_blob = np.array(embedding, dtype=np.float32).tobytes()

        await self._db.execute(
            "INSERT INTO visual_memory"
            " (id, screenshot_hash, embedding, description, context, actions_taken, pinned, created_at)"
            " VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
            (
                entry_id,
                screenshot.hash,
                embedding_blob,
                description,
                context,
                json.dumps(actions or []),
                datetime.now(UTC).isoformat(),
            ),
        )
        await self._db.commit()
        await self.cleanup()
        return entry_id

    # ── search ──────────────────────────────────────────────────

    async def search_by_image(
        self, screenshot: Screenshot, top_k: int = 5
    ) -> list[MemorySearchResult]:
        """Search for similar screenshots by image content."""
        query_embedding = await self.generate_embedding(screenshot.image_bytes)
        return await self._search(query_embedding, top_k)

    async def search_by_text(self, query: str, top_k: int = 5) -> list[MemorySearchResult]:
        """Search by text description using CLIP text encoder."""
        query_embedding = await self.generate_text_embedding(query)
        return await self._search(query_embedding, top_k)

    async def _search(self, query_embedding: list[float], top_k: int) -> list[MemorySearchResult]:
        assert self._db is not None
        query_vec = np.array(query_embedding, dtype=np.float32)

        results: list[MemorySearchResult] = []
        async with self._db.execute(
            "SELECT id, screenshot_hash, embedding, description, context,"
            " actions_taken, pinned, created_at FROM visual_memory"
        ) as cursor:
            async for row in cursor:
                stored_vec = np.frombuffer(row[2], dtype=np.float32)
                sim = float(np.dot(query_vec, stored_vec))
                entry = VisualMemoryEntry(
                    id=row[0],
                    screenshot_hash=row[1],
                    embedding=stored_vec.tolist(),
                    description=row[3],
                    context=row[4],
                    actions_taken=json.loads(row[5]),
                    timestamp=datetime.fromisoformat(row[7]),
                    pinned=bool(row[6]),
                )
                results.append(MemorySearchResult(entry=entry, similarity=sim))

        results.sort(key=lambda r: r.similarity, reverse=True)
        return results[:top_k]

    async def get_actions_for_screen(self, screenshot: Screenshot) -> list[str] | None:
        """If we've seen a similar screen before, return the actions that worked."""
        results = await self.search_by_image(screenshot, top_k=1)
        if results and results[0].similarity > 0.85:
            return results[0].entry.actions_taken
        return None

    # ── maintenance ─────────────────────────────────────────────

    async def cleanup(self) -> int:
        """Remove oldest non-pinned entries when exceeding max_entries. Returns count removed."""
        assert self._db is not None

        async with self._db.execute("SELECT COUNT(*) FROM visual_memory") as cursor:
            count = (await cursor.fetchone())[0]

        if count <= self._max_entries:
            return 0

        to_remove = count - self._max_entries
        await self._db.execute(
            "DELETE FROM visual_memory WHERE id IN"
            " (SELECT id FROM visual_memory WHERE pinned = 0"
            " ORDER BY created_at ASC LIMIT ?)",
            (to_remove,),
        )
        await self._db.commit()
        logger.info("Cleaned up %d visual memory entries", to_remove)
        return to_remove
