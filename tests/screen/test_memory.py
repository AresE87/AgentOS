"""Tests for VisualMemory — CLIP model is always mocked (no 400 MB download)."""

from __future__ import annotations

import hashlib
import io
from datetime import UTC, datetime
from unittest.mock import AsyncMock, patch

import numpy as np
import pytest
from PIL import Image

from agentos.screen.memory import VisualMemory
from agentos.types import Screenshot

# ── Helpers ──────────────────────────────────────────────────────────


def _make_screenshot(width: int = 200, height: int = 150, color: tuple = (255, 0, 0)) -> Screenshot:
    """Build a Screenshot from a synthetic solid-color PIL image."""
    img = Image.new("RGB", (width, height), color=color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    image_bytes = buf.getvalue()
    return Screenshot(
        image_bytes=image_bytes,
        width=width,
        height=height,
        timestamp=datetime.now(UTC),
        region=None,
        hash=hashlib.sha256(image_bytes).hexdigest()[:16],
    )


def _normalized_vec(seed: int, dim: int = 512) -> list[float]:
    """Return a deterministic normalized vector for testing."""
    rng = np.random.default_rng(seed)
    vec = rng.standard_normal(dim).astype(np.float32)
    vec /= np.linalg.norm(vec)
    return vec.tolist()


@pytest.fixture
async def memory() -> VisualMemory:
    """VisualMemory wired to an in-memory SQLite database."""
    mem = VisualMemory(db_path=":memory:", max_entries=1000)
    await mem.initialize()
    return mem


# ── Tests ────────────────────────────────────────────────────────────


async def test_store_and_search(memory: VisualMemory) -> None:
    """Store an entry and find it via image search."""
    vec = _normalized_vec(42)
    shot = _make_screenshot()

    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        entry_id = await memory.store(shot, description="desktop with terminal open")
        results = await memory.search_by_image(shot, top_k=3)

    assert len(results) == 1
    assert results[0].entry.id == entry_id
    assert results[0].similarity == pytest.approx(1.0, abs=1e-5)
    assert results[0].entry.description == "desktop with terminal open"


async def test_search_by_text(memory: VisualMemory) -> None:
    """Store with image embedding, search with text embedding."""
    img_vec = _normalized_vec(10)
    text_vec = _normalized_vec(10)  # identical → similarity 1.0

    shot = _make_screenshot()
    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=img_vec):
        await memory.store(shot, description="login page")

    with patch.object(
        memory, "generate_text_embedding", new_callable=AsyncMock, return_value=text_vec
    ):
        results = await memory.search_by_text("login screen", top_k=3)

    assert len(results) == 1
    assert results[0].similarity == pytest.approx(1.0, abs=1e-5)
    assert results[0].entry.description == "login page"


async def test_similarity_ordering(memory: VisualMemory) -> None:
    """Three entries with known embeddings — search returns correct order."""
    vec_a = _normalized_vec(1)
    vec_b = _normalized_vec(2)
    vec_c = _normalized_vec(3)

    shot = _make_screenshot()
    # Store three entries with distinct embeddings
    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_a):
        id_a = await memory.store(shot, description="entry A")
    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_b):
        await memory.store(shot, description="entry B")
    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_c):
        await memory.store(shot, description="entry C")

    # Search with vec_a → A should be first (sim=1.0), then B or C
    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_a):
        results = await memory.search_by_image(shot, top_k=3)

    assert len(results) == 3
    assert results[0].entry.id == id_a
    assert results[0].similarity == pytest.approx(1.0, abs=1e-5)
    # B and C should have lower similarity
    assert results[1].similarity < 1.0
    assert results[2].similarity < 1.0


async def test_cleanup_removes_oldest() -> None:
    """With max_entries=2, storing a third entry removes the oldest unpinned."""
    mem = VisualMemory(db_path=":memory:", max_entries=2)
    await mem.initialize()

    vec = _normalized_vec(99)
    shot = _make_screenshot()

    with patch.object(mem, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        id_first = await mem.store(shot, description="first")
        await mem.store(shot, description="second")
        await mem.store(shot, description="third")

    # Should now have exactly 2 entries (first was cleaned up)
    with patch.object(mem, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        results = await mem.search_by_image(shot, top_k=10)

    assert len(results) == 2
    found_ids = {r.entry.id for r in results}
    assert id_first not in found_ids

    await mem.close()


async def test_cleanup_preserves_pinned() -> None:
    """Pinned entries survive cleanup even when they are the oldest."""
    mem = VisualMemory(db_path=":memory:", max_entries=2)
    await mem.initialize()

    vec = _normalized_vec(77)
    shot = _make_screenshot()

    with patch.object(mem, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        id_pinned = await mem.store(shot, description="pinned entry")

    # Pin the first entry
    assert mem._db is not None
    await mem._db.execute("UPDATE visual_memory SET pinned = 1 WHERE id = ?", (id_pinned,))
    await mem._db.commit()

    with patch.object(mem, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        await mem.store(shot, description="second")
        await mem.store(shot, description="third")

    # Pinned entry should survive; total might be 3 because only unpinned are eligible
    with patch.object(mem, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        results = await mem.search_by_image(shot, top_k=10)

    found_ids = {r.entry.id for r in results}
    assert id_pinned in found_ids

    await mem.close()


async def test_get_actions_for_screen(memory: VisualMemory) -> None:
    """When a similar screen exists, get_actions_for_screen returns its actions."""
    vec = _normalized_vec(55)
    shot = _make_screenshot()
    actions = ["click OK button", "wait 2 seconds"]

    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec):
        await memory.store(shot, description="dialog box", actions=actions)
        result = await memory.get_actions_for_screen(shot)

    assert result == actions


async def test_get_actions_no_match(memory: VisualMemory) -> None:
    """With no similar screen (low similarity), returns None."""
    vec_stored = _normalized_vec(100)
    shot_stored = _make_screenshot()

    with patch.object(
        memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_stored
    ):
        await memory.store(shot_stored, description="settings page")

    # Use a very different embedding for the query (negate → sim ~ -1.0)
    vec_query = [-x for x in vec_stored]
    shot_query = _make_screenshot(color=(0, 0, 255))

    with patch.object(memory, "generate_embedding", new_callable=AsyncMock, return_value=vec_query):
        result = await memory.get_actions_for_screen(shot_query)

    assert result is None


async def test_embedding_roundtrip(memory: VisualMemory) -> None:
    """Embedding stored as BLOB then read back must match exactly."""
    original_vec = _normalized_vec(42)
    shot = _make_screenshot()

    with patch.object(
        memory, "generate_embedding", new_callable=AsyncMock, return_value=original_vec
    ):
        await memory.store(shot, description="roundtrip test")
        results = await memory.search_by_image(shot, top_k=1)

    stored_embedding = results[0].entry.embedding
    np.testing.assert_array_almost_equal(stored_embedding, original_vec, decimal=6)
