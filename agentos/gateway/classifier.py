"""Task classifier — determines type, complexity, and LLM tier for a task.

BaseClassifier defines the interface; RuleBasedClassifier implements
keyword + regex heuristics with bilingual (EN/ES) support.
"""

from __future__ import annotations

import re
from abc import ABC, abstractmethod

from agentos.types import LLMTier, TaskClassification, TaskInput, TaskType

# ── Keyword sets (bilingual EN + ES) ─────────────────────────────────

_VISION_KEYWORDS: set[str] = {
    "screenshot",
    "screen",
    "pantalla",
    "captura",
    "click",
    "clic",
    "button",
    "botón",
    "boton",
    "window",
    "ventana",
    "ui",
    "interfaz",
}

_CODE_KEYWORDS: set[str] = {
    "code",
    "código",
    "codigo",
    "function",
    "función",
    "funcion",
    "variable",
    "debug",
    "error",
    "bug",
    "compile",
    "compilar",
    "script",
    "programa",
    "class",
    "clase",
    "import",
    "module",
    "módulo",
    "algorithm",
    "algoritmo",
    "database",
    "sql",
    "query",
    "api",
    "endpoint",
    "rest",
    "graphql",
    "git",
    "commit",
    "push",
    "pull",
    "merge",
    "branch",
    "deploy",
    "deployment",
    "server",
    "servidor",
    "docker",
    "container",
    "kubernetes",
    "npm",
    "pip",
    "cargo",
    "package",
    "paquete",
    "library",
    "framework",
    "test",
    "testing",
    "unittest",
    "pytest",
    "refactor",
    "optimize",
    "optimizar",
}

_DATA_KEYWORDS: set[str] = {
    "spreadsheet",
    "planilla",
    "excel",
    "csv",
    "tsv",
    "datos",
    "table",
    "tabla",
    "column",
    "columna",
    "row",
    "fila",
    "chart",
    "gráfico",
    "grafico",
    "graph",
    "statistics",
    "estadísticas",
    "average",
    "promedio",
    "sum",
    "suma",
    "count",
    "contar",
    "filter",
    "filtrar",
    "sort",
    "ordenar",
    "pivot",
    "aggregate",
    "agrupar",
    "analysis",
    "análisis",
    "report",
    "reporte",
    "dashboard",
    "kpi",
    "metric",
    "métrica",
    "percentage",
    "porcentaje",
    "calculate",
    "calcular",
    "formula",
    "fórmula",
    "registros",
}

_GENERATION_KEYWORDS: set[str] = {
    "generate image",
    "generar imagen",
    "create image",
    "crear imagen",
    "draw",
    "dibujar",
    "logo",
    "illustration",
    "ilustración",
    "poster",
    "banner",
    "thumbnail",
    "wallpaper",
    "avatar",
    "generate art",
    "arte generativo",
    "midjourney",
    "dall-e",
    "stable diffusion",
    "flux",
}

# ── Regex patterns ────────────────────────────────────────────────────

_VISION_PATTERNS: list[re.Pattern[str]] = [
    re.compile(r"(?:take|toma|capture|captura).*(?:screenshot|captura|pantalla)", re.I),
    re.compile(r"(?:what|qué|que).*(?:screen|pantalla)", re.I),
    re.compile(r"(?:click|clic|press|presiona).*(?:button|botón|boton)", re.I),
    re.compile(r"\bnavigate\s+to\b.*\band\s+(?:change|click|select|set)\b", re.I),
]

_CODE_PATTERNS: list[re.Pattern[str]] = [
    re.compile(r"```[\s\S]*```"),
    re.compile(
        r"(?:write|escribe|create|crea|make|haz).*(?:code|código|function|función|script|programa|class)",
        re.I,
    ),
    re.compile(r"(?:fix|arregla|debug|solve|resuelve).*(?:bug|error|issue|problema)", re.I),
    re.compile(
        r"(?:how to|cómo|como).*(?:implement|implementar|code|programar|build|construir)", re.I
    ),
    re.compile(r"\b(?:def |class |import |from .+ import|return |async |await )", re.I),
    re.compile(r"(?:\.py|\.js|\.ts|\.rs|\.go|\.java|\.cpp|\.rb)\b"),
    re.compile(r"\b(?:list\s+files|ls\s|mkdir|chmod|curl\s)", re.I),
    re.compile(r"\breview\s+(?:this\s+)?code\b", re.I),
    re.compile(r"\bREST\s+API\b", re.I),
]

_DATA_PATTERNS: list[re.Pattern[str]] = [
    re.compile(
        r"(?:analyze|analiza|process|procesa).*(?:data|datos|csv|excel|spreadsheet|planilla)", re.I
    ),
    re.compile(
        r"(?:create|crea|make|haz|generate|genera).*(?:chart|gráfico|report|reporte|dashboard)",
        re.I,
    ),
    re.compile(r"(?:how many|cuántos|cuantos|what percentage|qué porcentaje)", re.I),
    re.compile(r"\b(?:pivot\s+table|standard\s+deviation)\b", re.I),
    re.compile(r"\bcolumn\s+[A-Z]\b", re.I),
]

_GENERATION_PATTERNS: list[re.Pattern[str]] = [
    re.compile(
        r"(?:generate|genera|create|crea|make|haz|draw|dibuja)\s+(?:an?\s+)?(?:image|imagen|illustration|ilustración|art|arte)\b",
        re.I,
    ),
    re.compile(
        r"(?:design|diseña)\s+(?:an?\s+)?(?:poster|banner|logo|image|imagen|illustration|ilustración)\b",
        re.I,
    ),
]

# ── Complexity helper patterns ────────────────────────────────────────

_SUBTASK_CONNECTORS: list[re.Pattern[str]] = [
    re.compile(r"\band\s+then\b", re.I),
    re.compile(r"\by\s+después\b", re.I),
    re.compile(r"\balso\b", re.I),
    re.compile(r"\btambién\b", re.I),
    re.compile(r"\bfirst\b.*\bthen\b", re.I | re.S),
    re.compile(r"\bprimero\b.*\bluego\b", re.I | re.S),
    re.compile(r"^\s*[-*•]\s", re.M),
    re.compile(r"^\s*\d+\.\s", re.M),
]

_COMMA_LIST_PATTERN = re.compile(r",\s*(?:and\s+|y\s+)?(?=\w)", re.I)

_CONDITIONAL_PATTERNS: list[re.Pattern[str]] = [
    re.compile(r"\bif\b", re.I),
    re.compile(r"\bwhen\b", re.I),
    re.compile(r"\bcuando\b", re.I),
    re.compile(r"\bunless\b", re.I),
    re.compile(r"\ba menos que\b", re.I),
    re.compile(r"\bdepending\b", re.I),
    re.compile(r"\bdependiendo\b", re.I),
]

_TOOL_REFERENCES: list[re.Pattern[str]] = [
    re.compile(r"\b(?:email|correo|e-mail)\b", re.I),
    re.compile(r"\b(?:database|base\s+de\s+datos|sql)\b", re.I),
    re.compile(r"\b(?:api|endpoint|rest)\b", re.I),
    re.compile(r"\b(?:docker|kubernetes|k8s)\b", re.I),
    re.compile(r"\b(?:git|github|gitlab)\b", re.I),
    re.compile(r"\b(?:csv|excel|spreadsheet|planilla)\b", re.I),
    re.compile(r"\b(?:chart|gráfico|report|reporte)\b", re.I),
    re.compile(r"\b(?:authentication|auth|autenticación)\b", re.I),
    re.compile(r"\b(?:presentation|presentación|board)\b", re.I),
]


def _has_action_verb(text: str) -> bool:
    pattern = re.compile(
        r"\b(?:write|read|create|delete|update|build|fix|debug|run|execute|send|"
        r"generate|analyze|calculate|deploy|test|install|open|close|click|take|"
        r"list|sort|filter|process|prepare|compare|review|identify|summarize|"
        r"highlight|suggest|translate|navigate|change|design|"
        r"escribe|lee|crea|borra|actualiza|construye|arregla|ejecuta|envía|"
        r"genera|analiza|calcula|despliega|prueba|instala|abre|cierra|haz)\b",
        re.I,
    )
    return bool(pattern.search(text))


class BaseClassifier(ABC):
    """Abstract base for task classifiers."""

    @abstractmethod
    async def classify(self, task_input: TaskInput) -> TaskClassification:
        """Classify a task and return type, complexity, tier, confidence."""


class RuleBasedClassifier(BaseClassifier):
    """Heuristic classifier using keywords and regex patterns."""

    async def classify(self, task_input: TaskInput) -> TaskClassification:
        text = task_input.text.strip()

        if not text:
            return TaskClassification(
                task_type=TaskType.TEXT,
                complexity=1,
                tier=LLMTier.CHEAP,
                confidence=0.40,
                reasoning="Empty input — defaulting to TEXT.",
            )

        task_type, confidence, reasoning = self._detect_type(text)
        complexity = self._compute_complexity(text)
        tier = self._map_tier(complexity)

        return TaskClassification(
            task_type=task_type,
            complexity=complexity,
            tier=tier,
            confidence=confidence,
            reasoning=reasoning,
        )

    def _detect_type(self, text: str) -> tuple[TaskType, float, str]:
        text_lower = text.lower()

        # Check GENERATION patterns first (to catch "create an image" before VISION grabs "image")
        gen_conf, gen_reason = self._check_match(
            text, text_lower, _GENERATION_KEYWORDS, _GENERATION_PATTERNS, "generation"
        )
        if gen_conf >= 0.85:
            code_conf, _ = self._check_match(
                text, text_lower, _CODE_KEYWORDS, _CODE_PATTERNS, "code"
            )
            if code_conf >= 0.85:
                return TaskType.CODE, 0.60, "Ambiguous (code+generation); chose code."
            return TaskType.GENERATION, gen_conf, gen_reason

        # Standard priority: VISION > CODE > DATA > GENERATION > TEXT
        vis_conf, vis_reason = self._check_match(
            text, text_lower, _VISION_KEYWORDS, _VISION_PATTERNS, "vision"
        )
        code_conf, code_reason = self._check_match(
            text, text_lower, _CODE_KEYWORDS, _CODE_PATTERNS, "code"
        )
        data_conf, data_reason = self._check_match(
            text, text_lower, _DATA_KEYWORDS, _DATA_PATTERNS, "data"
        )

        matches: list[tuple[TaskType, float, str]] = []
        if vis_conf > 0:
            matches.append((TaskType.VISION, vis_conf, vis_reason))
        if code_conf > 0:
            matches.append((TaskType.CODE, code_conf, code_reason))
        if data_conf > 0:
            matches.append((TaskType.DATA, data_conf, data_reason))
        if gen_conf > 0:
            matches.append((TaskType.GENERATION, gen_conf, gen_reason))

        if not matches:
            return TaskType.TEXT, 0.70, "No specific type matched — defaulting to TEXT."
        if len(matches) == 1:
            return matches[0]

        best = matches[0]
        return best[0], 0.60, f"Ambiguous ({len(matches)} types matched); chose {best[0].value}."

    def _check_match(
        self,
        text: str,
        text_lower: str,
        keywords: set[str],
        patterns: list[re.Pattern[str]],
        label: str,
    ) -> tuple[float, str]:
        for pat in patterns:
            if pat.search(text):
                return 0.90, f"Regex match for {label}: {pat.pattern!r}."
        for kw in sorted(keywords, key=len, reverse=True):
            if " " in kw:
                if kw in text_lower:
                    return 0.85, f"Multi-word keyword match for {label}: {kw!r}."
            elif re.search(rf"\b{re.escape(kw)}\b", text_lower):
                return 0.80, f"Keyword match for {label}: {kw!r}."
        return 0.0, ""

    def _compute_complexity(self, text: str) -> int:
        score = 1

        # Length
        if len(text) > 500:
            score += 2
        elif len(text) > 200:
            score += 1

        # Subtask connectors (explicit multi-step markers)
        subtask_count = sum(1 for pat in _SUBTASK_CONNECTORS if pat.search(text))

        # Count commas between clauses (only meaningful if 2+ commas = 3+ items)
        comma_items = len(_COMMA_LIST_PATTERN.findall(text))
        # Count "and" connecting clauses (English only — "y" is too common in Spanish)
        and_count = len(re.findall(r"\band\b", text, re.I))

        total_subtasks = subtask_count + (comma_items if comma_items >= 2 else 0) + and_count
        if total_subtasks >= 3:
            score += 2
        elif total_subtasks >= 1:
            score += 1

        # Conditionals
        if any(pat.search(text) for pat in _CONDITIONAL_PATTERNS):
            score += 1

        # Multiple tools / systems referenced
        tool_count = sum(1 for pat in _TOOL_REFERENCES if pat.search(text))
        if tool_count >= 2:
            score += 1

        # Vague / abstract
        if not _has_action_verb(text) and text.endswith("?"):
            score += 1

        return max(1, min(5, score))

    @staticmethod
    def _map_tier(complexity: int) -> LLMTier:
        if complexity <= 2:
            return LLMTier.CHEAP
        if complexity == 3:
            return LLMTier.STANDARD
        return LLMTier.PREMIUM
