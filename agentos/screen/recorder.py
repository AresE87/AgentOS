"""Step recorder for generating visual playbooks from user actions (AOS-016)."""

from __future__ import annotations

import shutil
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

import yaml

from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from agentos.screen.capture import ScreenCapture

logger = get_logger("screen.recorder")


@dataclass
class RecordedStep:
    """A single recorded step with screenshot and metadata."""

    step_number: int
    image_path: str
    annotation: str | None = None
    trigger: str = "manual"  # "click", "enter", "window_change", "manual"
    timestamp: datetime = field(default_factory=lambda: datetime.now(UTC))


@dataclass
class Recording:
    """A complete recording session."""

    id: str
    name: str
    steps: list[RecordedStep]
    started_at: datetime
    ended_at: datetime | None = None
    total_duration_ms: float = 0.0


class StepRecorder:
    """Records user actions step by step to generate visual playbooks."""

    def __init__(
        self,
        capture: ScreenCapture,
        output_dir: Path = Path("./recordings"),
    ) -> None:
        self._capture = capture
        self._output_dir = output_dir
        self._recording: Recording | None = None
        self._is_recording = False

    async def start_recording(self, name: str = "recording") -> str:
        """Start a new recording session. Returns recording ID."""
        recording_id = uuid.uuid4().hex[:12]
        self._recording = Recording(
            id=recording_id,
            name=name,
            steps=[],
            started_at=datetime.now(UTC),
        )
        self._is_recording = True

        # Create output directory
        rec_dir = self._output_dir / recording_id / "steps"
        rec_dir.mkdir(parents=True, exist_ok=True)

        logger.info("Recording started: %s (%s)", name, recording_id)
        return recording_id

    async def stop_recording(self) -> Recording:
        """Stop recording and return the recording data."""
        if not self._recording:
            msg = "No active recording"
            raise RuntimeError(msg)

        self._is_recording = False
        self._recording.ended_at = datetime.now(UTC)
        elapsed = (self._recording.ended_at - self._recording.started_at).total_seconds() * 1000
        self._recording.total_duration_ms = elapsed

        logger.info(
            "Recording stopped: %s (%d steps)",
            self._recording.name,
            len(self._recording.steps),
        )
        return self._recording

    async def capture_step(
        self,
        trigger: str = "manual",
        annotation: str | None = None,
    ) -> RecordedStep:
        """Capture a single step (screenshot + metadata)."""
        if not self._recording or not self._is_recording:
            msg = "Not recording"
            raise RuntimeError(msg)

        screenshot = await self._capture.capture_full()
        step_num = len(self._recording.steps) + 1

        # Save screenshot to disk
        rec_dir = self._output_dir / self._recording.id / "steps"
        filename = f"{step_num:02d}-{trigger}.png"
        filepath = rec_dir / filename
        filepath.write_bytes(screenshot.image_bytes)

        step = RecordedStep(
            step_number=step_num,
            image_path=str(filepath),
            annotation=annotation,
            trigger=trigger,
        )

        # Save annotation if provided
        if annotation:
            md_path = rec_dir / f"{step_num:02d}-{trigger}.md"
            md_path.write_text(annotation, encoding="utf-8")

        self._recording.steps.append(step)
        logger.info("Step %d captured (%s)", step_num, trigger)
        return step

    async def generate_playbook(self, recording: Recording, output_path: Path) -> Path:
        """Generate a Context Folder from a recording."""
        output_path.mkdir(parents=True, exist_ok=True)

        # Generate playbook.md
        lines = [
            f"# {recording.name}\n",
            "Recorded playbook with visual steps.\n",
            "## Steps\n",
        ]
        for step in recording.steps:
            desc = step.annotation or f"Step {step.step_number} ({step.trigger})"
            lines.append(f"{step.step_number}. {desc}\n")

        (output_path / "playbook.md").write_text("\n".join(lines), encoding="utf-8")

        # Generate config.yaml
        config = {
            "name": recording.name,
            "description": f"Recorded playbook with {len(recording.steps)} steps",
            "tier": 2,
            "timeout": 300,
            "permissions": ["screen"],
        }
        (output_path / "config.yaml").write_text(
            yaml.dump(config, default_flow_style=False),
            encoding="utf-8",
        )

        # Copy steps directory
        steps_dir = output_path / "steps"
        steps_dir.mkdir(exist_ok=True)
        src_steps = self._output_dir / recording.id / "steps"
        if src_steps.exists():
            for f in src_steps.iterdir():
                shutil.copy2(f, steps_dir / f.name)

        logger.info("Playbook generated at %s", output_path)
        return output_path

    @property
    def is_recording(self) -> bool:
        """Whether a recording session is active."""
        return self._is_recording

    def get_recorded_steps(self) -> list[RecordedStep]:
        """Return a copy of the steps recorded so far."""
        if not self._recording:
            return []
        return list(self._recording.steps)
