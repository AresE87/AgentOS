# ML/AI Design: AOS-003 — LLM Gateway — Clasificador de tareas (reglas v1)

**Ticket:** AOS-003
**Rol:** ML/AI Engineer
**Input:** Especificación de producto (sección 3.2), AOS-001 Architecture (types.py), AOS-002 API Contract (TaskClassification)
**Fecha:** Marzo 2026

---

## Objetivo

Implementar un clasificador de tareas basado en reglas puras (zero dependencies) que analiza cada mensaje entrante y determina:

1. **Tipo de tarea** (TaskType): text, code, vision, generation, data
2. **Complejidad** (1–5): cuán difícil es la tarea
3. **Tier LLM** (LLMTier): cheap/standard/premium — derivado del tipo + complejidad
4. **Confianza** (0.0–1.0): cuán seguro está el clasificador
5. **Razonamiento** (str): explicación legible de por qué clasificó así

El clasificador v1 es **puramente basado en reglas** (regex, keywords, heurísticas). No usa modelos ML, no hace API calls, no tiene dependencias externas. Debe ejecutarse en < 10ms.

---

## Interface (ya definida en types.py)

```python
async def classify(task_input: TaskInput) -> TaskClassification
```

Donde `TaskClassification` tiene: `task_type`, `complexity`, `tier`, `confidence`, `reasoning`.

---

## Estrategia de clasificación: Strategy Pattern

La interfaz debe ser intercambiable para que en v2 se pueda reemplazar por un clasificador ML.

```python
from abc import ABC, abstractmethod

class BaseClassifier(ABC):
    @abstractmethod
    async def classify(self, task_input: TaskInput) -> TaskClassification:
        ...

class RuleBasedClassifier(BaseClassifier):
    """v1: Reglas puras. Zero dependencies."""
    ...

# Futuro v2:
# class MLClassifier(BaseClassifier):
#     """v2: Modelo ML entrenado con datos de uso."""
#     ...
```

---

## Reglas de clasificación de tipo (TaskType)

El clasificador aplica las reglas en orden de prioridad. La PRIMERA regla que matchea gana.

### Regla 1: VISION — Referencias a pantalla o imágenes

**Condición:** El texto contiene keywords relacionados con lo visual.

**Keywords (case-insensitive):**
```
screenshot, screen, pantalla, captura, what's on screen, que hay en pantalla,
look at, mira la, see the, ve la, UI, interfaz, botón, button, click, clic,
window, ventana, icon, ícono, image, imagen, photo, foto, display, mostrar pantalla
```

**Patterns (regex):**
```
r"(?:take|toma|capture|captura).*(?:screenshot|captura|pantalla)"
r"(?:what|qué|que).*(?:screen|pantalla|see|ver)"
r"(?:click|clic|press|presiona).*(?:button|botón|boton)"
```

**Confianza:** 0.85 si matchea keyword, 0.95 si matchea pattern.

---

### Regla 2: CODE — Código o tareas técnicas

**Condición:** El texto contiene syntax de código, keywords técnicos, o pide código.

**Keywords:**
```
code, código, function, función, variable, debug, error, bug, compile, compilar,
script, programa, class, clase, import, module, módulo, algorithm, algoritmo,
database, base de datos, SQL, query, consulta, API, endpoint, REST, GraphQL,
git, commit, push, pull, merge, branch, deploy, deployment, server, servidor,
docker, container, kubernetes, npm, pip, cargo, package, paquete, library,
framework, test, testing, unittest, pytest, refactor, optimize, optimizar
```

**Patterns:**
```
r"```[\s\S]*```"                           # code blocks
r"(?:write|escribe|create|crea|make|haz).*(?:code|código|function|función|script|programa|class)"
r"(?:fix|arregla|debug|solve|resuelve).*(?:bug|error|issue|problema)"
r"(?:how to|cómo|como).*(?:implement|implementar|code|programar|build|construir)"
r"\b(?:def|class|import|from|return|async|await|function|const|let|var)\b"  # syntax keywords
r"(?:\.py|\.js|\.ts|\.rs|\.go|\.java|\.cpp|\.rb)\b"                        # file extensions
```

**Confianza:** 0.90 si matchea syntax, 0.80 si matchea keywords.

---

### Regla 3: DATA — Datos tabulares y análisis

**Condición:** El texto referencia datos, planillas, CSV, análisis numérico.

**Keywords:**
```
spreadsheet, planilla, excel, csv, tsv, data, datos, table, tabla, column, columna,
row, fila, chart, gráfico, graph, grafo, statistics, estadísticas, average, promedio,
sum, suma, count, contar, filter, filtrar, sort, ordenar, pivot, aggregate, agrupar,
analysis, análisis, report, reporte, dashboard, KPI, metric, métrica, percentage,
porcentaje, calculate, calcular, formula, fórmula
```

**Patterns:**
```
r"(?:analyze|analiza|process|procesa).*(?:data|datos|csv|excel|spreadsheet|planilla)"
r"(?:create|crea|make|haz|generate|genera).*(?:chart|gráfico|report|reporte|dashboard)"
r"(?:how many|cuántos|cuantos|what percentage|qué porcentaje)"
```

**Confianza:** 0.80.

---

### Regla 4: GENERATION — Crear contenido no-código

**Condición:** El texto pide crear imágenes u otro contenido generativo.

**Keywords:**
```
generate image, generar imagen, create image, crear imagen, draw, dibujar,
design, diseñar, logo, illustration, ilustración, poster, banner, thumbnail,
wallpaper, avatar, icon design, generate art, arte generativo, midjourney,
dall-e, stable diffusion, flux
```

**Patterns:**
```
r"(?:generate|genera|create|crea|make|haz|draw|dibuja|design|diseña).*(?:image|imagen|logo|poster|banner|illustration|ilustración|art|arte)"
```

**Confianza:** 0.85.

---

### Regla 5: TEXT — Todo lo demás (default)

**Condición:** Si ninguna de las reglas anteriores matchea, es TEXT.

**Confianza:** 0.70 (baja porque es el default catch-all).

---

## Reglas de complejidad (1–5)

La complejidad se calcula independientemente del tipo, analizando la ESTRUCTURA del mensaje.

| Factor | Peso | Cómo se mide |
|--------|------|-------------|
| Longitud del texto | +1 si > 200 chars, +1 más si > 500 chars | `len(text)` |
| Sub-tareas implícitas | +1 por cada conector de múltiples tareas | Contar: "and then", "y después", "also", "también", "first...then", "primero...luego", bullets/números |
| Condicionales | +1 si hay lógica condicional | "if", "si", "when", "cuando", "unless", "a menos que", "depending", "dependiendo" |
| Múltiples herramientas | +1 si referencia más de un sistema | Contar entidades: archivos + comandos + URLs + apps mencionadas |
| Ambigüedad / abstracción | +1 si el pedido es vago o abstracto | Ausencia de verbos de acción concretos, preguntas abiertas ("what do you think", "qué opinas") |

**Fórmula:**
```python
base = 1
complexity = base + length_score + subtask_score + conditional_score + tool_score + ambiguity_score
complexity = min(complexity, 5)  # clamp to 1-5
```

**Ejemplos calibrados:**

| Input | Tipo | Complejidad | Reasoning |
|-------|------|-------------|-----------|
| "hello" | TEXT | 1 | Short greeting, no subtasks |
| "what time is it?" | TEXT | 1 | Simple question |
| "list files in /home" | CODE | 1 | Single CLI command |
| "summarize this article about AI" | TEXT | 2 | Moderate length, single task |
| "write a Python function that sorts a list and add unit tests" | CODE | 3 | Two subtasks (function + tests) |
| "research competitor pricing, create a spreadsheet comparison, and write a summary report" | DATA | 4 | Three subtasks, multiple tools |
| "analyze our Q3 financials, compare with Q2, identify trends, create charts, and prepare a board presentation with recommendations" | DATA | 5 | Five subtasks, multiple tools, conditional analysis |

---

## Mapeo de complejidad a tier

Directo, desde la spec:

| Complejidad | Tier |
|-------------|------|
| 1 | CHEAP (1) |
| 2 | CHEAP (1) |
| 3 | STANDARD (2) |
| 4 | PREMIUM (3) |
| 5 | PREMIUM (3) |

---

## Soporte bilingüe

Todas las reglas deben funcionar en **inglés Y español**. Los keywords y patterns incluyen ambos idiomas. El clasificador NO detecta idioma — simplemente incluye ambos sets de keywords.

---

## Estructura de archivos

```
agentos/gateway/classifier.py    # BaseClassifier + RuleBasedClassifier
tests/gateway/test_classifier.py # 30+ test cases
```

---

## Test cases requeridos (mínimo 30)

### TEXT (8 tests)
1. "hello" → TEXT, complexity=1
2. "what time is it?" → TEXT, complexity=1
3. "explain quantum computing" → TEXT, complexity=2
4. "hola, cómo estás?" → TEXT, complexity=1
5. "write me an email to my boss explaining I'll be late" → TEXT, complexity=2
6. "summarize the following article and highlight key points and suggest follow-up questions" → TEXT, complexity=3
7. "translate this paragraph to Spanish" → TEXT, complexity=2
8. "what do you think about the future of remote work and how it will affect urban planning?" → TEXT, complexity=3

### CODE (8 tests)
9. "list files in /home" → CODE, complexity=1
10. "write a Python function to sort a list" → CODE, complexity=2
11. "```python\ndef hello():\n    pass\n```\nfix this function" → CODE, complexity=2
12. "debug this error: ModuleNotFoundError" → CODE, complexity=2
13. "create a REST API with authentication and database connection" → CODE, complexity=4
14. "write a script that reads a CSV, processes data, generates a report, and sends it by email" → CODE, complexity=4
15. "escribe una función en Python que ordene diccionarios" → CODE, complexity=2
16. "review this code and suggest improvements for performance and readability" → CODE, complexity=3

### VISION (4 tests)
17. "take a screenshot" → VISION, complexity=1
18. "what's on my screen?" → VISION, complexity=2
19. "click the submit button" → VISION, complexity=1
20. "navigate to settings and change the theme to dark mode" → VISION, complexity=3

### DATA (5 tests)
21. "analyze this CSV file" → DATA, complexity=2
22. "create a chart showing monthly revenue" → DATA, complexity=2
23. "calculate the average and standard deviation of column B" → DATA, complexity=2
24. "process the sales data, create a pivot table, generate charts, and prepare a report" → DATA, complexity=4
25. "cuántos registros hay en la planilla?" → DATA, complexity=1

### GENERATION (3 tests)
26. "generate a logo for my company" → GENERATION, complexity=2
27. "create an image of a sunset over mountains" → GENERATION, complexity=2
28. "design a poster for our event with illustrations and typography" → GENERATION, complexity=3

### Edge cases (2 tests)
29. "" (empty string) → TEXT, complexity=1, confidence < 0.5
30. "write code to generate an image from data in a spreadsheet" → CODE (dominant), complexity=4 (multi-tool, multi-type — code wins because action is "write code")

---

## Regla de desempate

Si un mensaje matchea múltiples tipos, el ORDEN DE PRIORIDAD decide:
1. VISION (más específico, más costoso de equivocarse)
2. CODE (segundo más específico)
3. DATA
4. GENERATION
5. TEXT (siempre es fallback)

Si matchea VISION y CODE, gana VISION. Si matchea CODE y DATA, gana CODE.
La confianza se reduce a 0.60 cuando hay ambigüedad entre tipos.

---

## Performance target

- **Latencia:** < 10ms por clasificación (puro Python, sin I/O)
- **Memoria:** < 1 MB adicional (solo strings en memoria)
- **Zero dependencies:** Solo stdlib + types.py de AgentOS

---

## Extensibilidad futura (v2)

La interfaz `BaseClassifier` permite:
- **v2a:** Reemplazar con un modelo fine-tuned (ej: DistilBERT clasificador)
- **v2b:** Hybrid: reglas primero, ML si confidence < 0.6
- **v2c:** Feedback loop: si el usuario corrige el tier, ajustar las reglas/pesos
