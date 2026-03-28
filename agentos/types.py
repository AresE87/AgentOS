"""Shared data types for AgentOS.

All cross-module data structures live here. Using frozen dataclasses
for immutability where possible. These types ARE the contract between modules.
"""

from __future__ import annotations

import enum
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime

# ─── Enums ───────────────────────────────────────────────────────────


class TaskType(enum.StrEnum):
    """Types of tasks the agent can handle."""

    TEXT = "text"
    CODE = "code"
    VISION = "vision"
    GENERATION = "generation"
    DATA = "data"


class LLMTier(int, enum.Enum):
    """Budget tiers for LLM routing."""

    CHEAP = 1
    STANDARD = 2
    PREMIUM = 3


class TaskStatus(enum.StrEnum):
    """Lifecycle status of a task."""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


class ModelProvider(enum.StrEnum):
    """Supported LLM providers."""

    ANTHROPIC = "anthropic"
    OPENAI = "openai"
    GOOGLE = "google"
    LOCAL = "local"


# ─── LLM Gateway Types ──────────────────────────────────────────────


@dataclass(frozen=True)
class ModelConfig:
    """Configuration for a specific model loaded from routing.yaml."""

    provider: ModelProvider
    model_id: str
    display_name: str
    cost_per_1m_input: float
    cost_per_1m_output: float
    max_tokens: int


@dataclass(frozen=True)
class LLMRequest:
    """High-level request to the LLM Gateway."""

    prompt: str
    tier: LLMTier
    task_type: TaskType
    system_prompt: str = ""
    max_tokens: int = 4096
    temperature: float = 0.7


@dataclass(frozen=True)
class LLMResponse:
    """Normalized response from any provider."""

    content: str
    model: str
    provider: str
    tokens_in: int
    tokens_out: int
    cost_estimate: float
    latency_ms: float


@dataclass(frozen=True)
class GatewayHealthStatus:
    """Result of a Gateway health check."""

    providers: dict[str, bool]
    available_models: int
    default_provider: str | None


# ─── Task Classification ────────────────────────────────────────────


@dataclass(frozen=True)
class TaskClassification:
    """Result of classifying a task."""

    task_type: TaskType
    complexity: int  # 1-5
    tier: LLMTier
    confidence: float  # 0.0-1.0
    reasoning: str


# ─── Task Processing ────────────────────────────────────────────────


def _generate_task_id() -> str:
    return uuid.uuid4().hex[:12]


@dataclass(frozen=True)
class TaskInput:
    """Input to the agent pipeline."""

    text: str
    source: str = "telegram"
    chat_id: str = ""
    task_id: str = field(default_factory=_generate_task_id)
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass(frozen=True)
class TaskResult:
    """Result of processing a task through the agent pipeline."""

    task_id: str
    input_text: str
    source: str
    status: TaskStatus
    classification: TaskClassification | None = None
    model_used: str | None = None
    provider: str | None = None
    tokens_in: int = 0
    tokens_out: int = 0
    cost_estimate: float = 0.0
    output_text: str = ""
    error_message: str = ""
    created_at: datetime = field(default_factory=lambda: datetime.now(UTC))
    completed_at: datetime | None = None
    duration_ms: float = 0.0


# ─── Execution Types ────────────────────────────────────────────────


class ExecutorType(enum.StrEnum):
    """Mode of execution."""

    CLI = "cli"
    API = "api"
    SCREEN = "screen"


@dataclass(frozen=True)
class ExecutionResult:
    """Result of executing a CLI command or screen action."""

    command: str
    exit_code: int
    stdout: str
    stderr: str
    duration_ms: float
    timed_out: bool = False
    executor_type: ExecutorType = ExecutorType.CLI


# ─── Screen Types ────────────────────────────────────────────────────


@dataclass(frozen=True)
class Screenshot:
    """A captured screenshot."""

    image_bytes: bytes
    width: int
    height: int
    timestamp: datetime
    region: tuple[int, int, int, int] | None = None  # (x, y, w, h) or None=full
    hash: str = ""


class ScreenActionType(enum.StrEnum):
    """Types of screen actions."""

    CLICK = "click"
    DOUBLE_CLICK = "double_click"
    RIGHT_CLICK = "right_click"
    DRAG = "drag"
    TYPE = "type"
    HOTKEY = "hotkey"
    SCROLL = "scroll"
    MOVE = "move"
    WAIT = "wait"
    PRESS_KEY = "press_key"


@dataclass(frozen=True)
class ScreenAction:
    """Record of an executed screen action."""

    action_type: ScreenActionType
    params: dict[str, object]
    timestamp: datetime
    success: bool
    duration_ms: float
    error: str | None = None


@dataclass(frozen=True)
class UIElement:
    """A UI element detected in a screenshot."""

    element_type: str  # "button", "input", "link", "menu", "text", "dropdown"
    label: str
    location: str  # "top-right", "center", etc.
    x: int = 0
    y: int = 0
    width: int = 0
    height: int = 0
    confidence: float = 0.0


@dataclass(frozen=True)
class ScreenAnalysis:
    """Result of analyzing a screenshot with a vision model."""

    description: str
    elements: list[UIElement]
    visible_text: str
    app_name: str | None = None
    screenshot_hash: str = ""
    model_used: str = ""
    tokens_used: int = 0
    cost: float = 0.0


# ─── Context Folder Types ───────────────────────────────────────────


@dataclass
class PlaybookConfig:
    """Configuration parsed from config.yaml in a context folder."""

    name: str
    description: str = ""
    tier: LLMTier = LLMTier.CHEAP
    timeout: int = 300
    permissions: list[str] = field(default_factory=list)
    allowed_commands: list[str] = field(default_factory=list)
    blocked_commands: list[str] = field(default_factory=list)


@dataclass
class StepRecord:
    """A step from a visual playbook (CFP v2)."""

    step_number: int
    image_path: str
    annotation: str | None = None
    has_embedding: bool = False


@dataclass
class ContextFolder:
    """Parsed context folder (playbook). Supports v1 and v2."""

    path: str
    config: PlaybookConfig
    instructions: str
    steps: list[StepRecord] = field(default_factory=list)
    templates: dict[str, str] = field(default_factory=dict)
    version: int = 1  # 1=text only, 2=has steps/


# ─── Cost Tracking Types ────────────────────────────────────────────


@dataclass(frozen=True)
class UsageSummary:
    """Summary of LLM usage for a period."""

    total_tokens_in: int
    total_tokens_out: int
    total_cost: float
    total_calls: int
    calls_by_provider: dict[str, int]
    calls_by_model: dict[str, int]
    success_rate: float
