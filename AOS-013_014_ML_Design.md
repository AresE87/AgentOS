# ML/AI Design: AOS-013 + AOS-014 — Vision Model Integration y Visual Memory (CLIP)

**Tickets:** AOS-013 (Vision), AOS-014 (CLIP)
**Rol:** ML/AI Engineer
**Input:** Especificación de producto (secciones 3.3, 4.2), AOS-002 API Contract, AOS-011/012 Architecture
**Fecha:** Marzo 2026

---

## PARTE 1: AOS-013 — Vision Model Integration

### Objetivo

Permitir al agente "ver" la pantalla enviando screenshots a un LLM de visión y recibiendo una descripción estructurada de lo que hay en pantalla.

### Componente

| Archivo | Responsabilidad |
|---------|-----------------|
| `executor/vision.py` | VisionAnalyzer: envía screenshots al LLM Gateway con prompts de visión |

### Interface: VisionAnalyzer

```python
@dataclass(frozen=True)
class ScreenAnalysis:
    """Resultado del análisis de un screenshot por el modelo de visión."""
    description: str                    # Descripción general de la pantalla
    ui_elements: list[UIElement]        # Elementos UI detectados
    visible_text: list[str]             # Texto visible en pantalla
    suggested_action: str | None        # Acción sugerida por el modelo
    confidence: float                   # 0.0-1.0
    model_used: str                     # Modelo que generó el análisis
    tokens_used: int
    cost: float


@dataclass(frozen=True)
class UIElement:
    """Un elemento de UI detectado en el screenshot."""
    element_type: str           # "button", "input", "link", "menu", "text", "image", "dropdown"
    label: str                  # Texto del elemento (ej: "Submit", "Search...")
    location: str               # Descripción de ubicación (ej: "top-right", "center")
    estimated_coords: tuple[int, int] | None  # (x, y) estimado para click


class VisionAnalyzer:
    """Analiza screenshots usando modelos de visión via LLM Gateway."""

    def __init__(self, gateway: LLMGateway) -> None:
        ...

    async def analyze_screen(self, screenshot: Screenshot, context: str = "") -> ScreenAnalysis:
        """Analiza qué hay en pantalla.

        Args:
            screenshot: Screenshot capturado.
            context: Contexto adicional (ej: "estoy intentando enviar un email").
        """
        ...

    async def find_element(self, screenshot: Screenshot, target: str) -> UIElement | None:
        """Busca un elemento específico en el screenshot.

        Args:
            target: Descripción del elemento (ej: "the Submit button", "el campo de email").
        """
        ...

    async def compare_screens(self, before: Screenshot, after: Screenshot) -> str:
        """Compara dos screenshots y describe qué cambió.

        Útil para verificar que una acción tuvo el efecto esperado.
        """
        ...
```

### Prompt templates

#### analyze_screen
```
You are analyzing a screenshot of a computer screen. Describe what you see in structured JSON format:

{
  "description": "Brief description of the screen content",
  "ui_elements": [
    {"type": "button|input|link|menu|text|dropdown", "label": "visible text", "location": "position description", "coords": [x, y]}
  ],
  "visible_text": ["text line 1", "text line 2"],
  "suggested_action": "what the user should do next based on context"
}

Context: {context}

Be precise about element locations. Use coordinates relative to the image dimensions.
Respond ONLY with valid JSON, no other text.
```

#### find_element
```
Find the UI element described as: "{target}"

Look at the screenshot and return the element's location as JSON:
{
  "found": true/false,
  "type": "button|input|link|...",
  "label": "visible text on element",
  "coords": [x, y],
  "confidence": 0.0-1.0
}

The coordinates should be the CENTER of the element, relative to image dimensions.
Respond ONLY with valid JSON.
```

#### compare_screens
```
Compare these two screenshots (before and after an action was performed).
Describe what changed between them in 1-2 sentences.
Focus on: new windows, dialogs, error messages, form state changes, navigation.
```

### Selección de modelo de visión

Los modelos de visión se seleccionan vía el routing table existente (task_type=VISION):

| Tier | Modelo preferido | Costo por screenshot (1024px) | Uso |
|------|-----------------|-------------------------------|-----|
| 1 (cheap) | Gemini Flash | ~$0.0001 | Verificaciones rápidas |
| 2 (standard) | GPT-4o | ~$0.003 | Análisis general |
| 3 (premium) | Claude Sonnet | ~$0.005 | Análisis detallado, elementos complejos |

### Compresión inteligente

Antes de enviar al LLM, el screenshot se optimiza:

```python
def prepare_for_llm(screenshot: Screenshot, tier: LLMTier) -> tuple[str, str]:
    """Prepara un screenshot para envío al LLM.

    Returns:
        (base64_string, media_type)
    """
    # Tier 1: agresivo — 512px, JPEG 70%
    # Tier 2: balanceado — 1024px, JPEG 85%
    # Tier 3: calidad — 1024px, PNG
```

---

## PARTE 2: AOS-014 — Visual Memory (CLIP)

### Objetivo

Crear un sistema de memoria visual que indexa screenshots con embeddings CLIP y permite búsqueda por similitud. Cuando el agente ve una pantalla desconocida, busca en la memoria el screenshot más parecido y sigue las instrucciones asociadas.

### Componente

| Archivo | Responsabilidad |
|---------|-----------------|
| `context/visual_memory.py` | VisualMemory: genera embeddings CLIP, almacena, busca por similitud |

### Modelo CLIP

**Modelo:** `openai/clip-vit-base-patch32` (ViT-B/32)
- Dimensión del embedding: 512
- Tamaño del modelo: ~340 MB
- Inference time (CPU): ~50ms por imagen
- Memoria: ~400 MB cargado

**Librería:** `transformers` + `torch` (CPU only)

```python
# Alternativa más ligera si torch es demasiado pesado:
# sentence-transformers con clip-ViT-B-32 — misma calidad, API más simple
```

### Interface: VisualMemory

```python
@dataclass
class VisualMemoryEntry:
    """Un screenshot indexado en la memoria visual."""
    id: str                     # UUID
    playbook_path: str          # Playbook al que pertenece
    step_number: int            # Número de paso (1, 2, 3...)
    image_path: str             # Path al archivo de imagen
    embedding: list[float]      # Vector CLIP de 512 dims
    annotation: str | None      # Anotación markdown del paso
    created_at: datetime


@dataclass(frozen=True)
class SearchResult:
    """Resultado de una búsqueda por similitud."""
    entry: VisualMemoryEntry
    similarity: float           # Cosine similarity 0.0-1.0
    rank: int                   # 1 = más similar


class VisualMemory:
    """Sistema de memoria visual basado en CLIP embeddings."""

    def __init__(self, store: TaskStore, model_name: str = "openai/clip-vit-base-patch32") -> None:
        ...

    async def load_model(self) -> None:
        """Carga el modelo CLIP. Se llama una vez al inicio.

        El modelo se descarga la primera vez (~340MB) y se cachea localmente.
        """
        ...

    async def generate_embedding(self, image_bytes: bytes) -> list[float]:
        """Genera el embedding CLIP de una imagen.

        Returns:
            Vector de 512 floats normalizado (L2 norm = 1.0).
        """
        ...

    async def index_screenshot(
        self,
        playbook_path: str,
        step_number: int,
        image_path: str,
        annotation: str | None = None,
    ) -> VisualMemoryEntry:
        """Indexa un screenshot: genera embedding y lo almacena."""
        ...

    async def search(self, query_image: bytes, top_k: int = 5, playbook_path: str | None = None) -> list[SearchResult]:
        """Busca los screenshots más similares a la imagen query.

        Args:
            query_image: Screenshot actual (bytes).
            top_k: Cuántos resultados retornar.
            playbook_path: Si se especifica, busca solo dentro de un playbook.

        Returns:
            Lista ordenada por similitud (mayor primero).
        """
        ...

    async def index_playbook_steps(self, playbook_path: str, steps_dir: Path) -> int:
        """Indexa todos los screenshots de un directorio steps/.

        Returns:
            Número de screenshots indexados.
        """
        ...

    def is_model_loaded(self) -> bool:
        """Verifica si el modelo CLIP está cargado en memoria."""
        ...
```

### Schema SQLite (tabla nueva)

```sql
CREATE TABLE IF NOT EXISTS visual_memory (
    id              TEXT PRIMARY KEY,
    playbook_path   TEXT NOT NULL,
    step_number     INTEGER NOT NULL,
    image_path      TEXT NOT NULL,
    embedding       BLOB NOT NULL,          -- 512 floats → 2048 bytes (float32)
    annotation      TEXT,
    created_at      TEXT NOT NULL,

    UNIQUE(playbook_path, step_number)
);

CREATE INDEX IF NOT EXISTS idx_vm_playbook ON visual_memory(playbook_path);
```

### Búsqueda por similitud (cosine similarity)

```python
import numpy as np

def cosine_similarity(a: np.ndarray, b: np.ndarray) -> float:
    """Calcula cosine similarity entre dos vectores normalizados.

    Si ambos están L2-normalizados: sim = dot(a, b)
    """
    return float(np.dot(a, b))


async def search(self, query_image: bytes, top_k: int = 5, ...) -> list[SearchResult]:
    # 1. Generar embedding del query
    query_embedding = await self.generate_embedding(query_image)
    query_vec = np.array(query_embedding, dtype=np.float32)

    # 2. Cargar todos los embeddings del playbook desde DB
    entries = await self._load_entries(playbook_path)

    # 3. Calcular similitud con cada uno
    results = []
    for entry in entries:
        stored_vec = np.frombuffer(entry.embedding_blob, dtype=np.float32)
        sim = cosine_similarity(query_vec, stored_vec)
        results.append((entry, sim))

    # 4. Ordenar por similitud y retornar top-k
    results.sort(key=lambda x: x[1], reverse=True)
    return [SearchResult(entry=e, similarity=s, rank=i+1) for i, (e, s) in enumerate(results[:top_k])]
```

**Nota:** Para < 10,000 screenshots, búsqueda bruta-force es suficiente (~5ms). Si escala, migrar a FAISS o Annoy.

### Almacenamiento de embeddings en SQLite

```python
# Guardar: float32 array → bytes
embedding_blob = np.array(embedding, dtype=np.float32).tobytes()  # 512 * 4 = 2048 bytes

# Cargar: bytes → float32 array
embedding = np.frombuffer(blob, dtype=np.float32)  # 512 floats
```

---

## Dependencias Python nuevas

```
torch >= 2.0          # CLIP inference (CPU only)
transformers >= 4.30  # Modelo CLIP
numpy >= 1.24         # Vectores y cosine similarity
```

**Alternativa ligera (si torch es demasiado pesado):**
```
sentence-transformers >= 2.7  # Incluye CLIP con API simple
numpy >= 1.24
```

---

## Test cases

### AOS-013 (Vision)
| # | Test | Expected |
|---|------|----------|
| 1 | analyze_screen con screenshot estático | Retorna ScreenAnalysis con elementos |
| 2 | find_element("Submit button") | Retorna UIElement con coords estimadas |
| 3 | compare_screens (dos screenshots diferentes) | Describe los cambios |
| 4 | Screenshot muy grande (4K) se comprime | Se redimensiona antes de enviar |
| 5 | Tier 1 usa JPEG agresivo | Formato = jpeg, quality < 80 |

### AOS-014 (CLIP)
| # | Test | Expected |
|---|------|----------|
| 1 | generate_embedding retorna 512-dim vector | len(embedding) == 512 |
| 2 | Embedding está normalizado (L2 norm ≈ 1.0) | np.linalg.norm ≈ 1.0 |
| 3 | Dos screenshots iguales → similarity > 0.99 | Verificar con misma imagen |
| 4 | Dos screenshots muy diferentes → similarity < 0.5 | Verificar con imágenes distintas |
| 5 | index_playbook_steps con 5 imágenes | 5 entries en DB |
| 6 | search retorna top-3 ordenado por similitud | Verificar orden |
| 7 | search filtrado por playbook_path | Solo retorna entries de ese playbook |
| 8 | Embedding round-trip: save → load → comparar | Identico después de save/load |

**Nota:** Tests de CLIP usan imágenes sintéticas generadas con Pillow (cuadrados de colores). No dependen del modelo real (se mockea).
