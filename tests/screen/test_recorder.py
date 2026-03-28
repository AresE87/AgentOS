"""Tests for StepRecorder (AOS-016)."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock

import pytest

from agentos.screen.recorder import Recording, StepRecorder
from agentos.types import Screenshot

# ── Helpers / Fixtures ────────────────────────────────────────────────


def _fake_screenshot() -> Screenshot:
    """Return a minimal synthetic Screenshot."""
    # 1x1 red PNG (valid minimal PNG bytes)
    import hashlib
    import io

    from PIL import Image

    img = Image.new("RGB", (100, 80), color=(255, 0, 0))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    image_bytes = buf.getvalue()
    return Screenshot(
        image_bytes=image_bytes,
        width=100,
        height=80,
        timestamp=MagicMock(),
        region=None,
        hash=hashlib.sha256(image_bytes).hexdigest()[:16],
    )


@pytest.fixture
def mock_capture() -> MagicMock:
    """A mock ScreenCapture whose capture_full returns a fake screenshot."""
    capture = MagicMock()
    capture.capture_full = AsyncMock(return_value=_fake_screenshot())
    return capture


@pytest.fixture
def recorder(mock_capture: MagicMock, tmp_path: object) -> StepRecorder:
    from pathlib import Path

    return StepRecorder(capture=mock_capture, output_dir=Path(str(tmp_path)))


# ── Tests ─────────────────────────────────────────────────────────────


async def test_start_recording(recorder: StepRecorder) -> None:
    """start_recording returns an ID and sets is_recording=True."""
    rec_id = await recorder.start_recording("my-test")
    assert isinstance(rec_id, str)
    assert len(rec_id) == 12
    assert recorder.is_recording is True


async def test_stop_recording(recorder: StepRecorder) -> None:
    """stop_recording returns a Recording with ended_at set."""
    await recorder.start_recording("stop-test")
    recording = await recorder.stop_recording()

    assert isinstance(recording, Recording)
    assert recording.ended_at is not None
    assert recording.total_duration_ms >= 0
    assert recorder.is_recording is False


async def test_capture_step(
    recorder: StepRecorder,
    mock_capture: MagicMock,
    tmp_path: object,
) -> None:
    """capture_step takes a screenshot, writes file, and adds to steps."""
    from pathlib import Path

    await recorder.start_recording("capture-test")
    step = await recorder.capture_step(trigger="click", annotation=None)

    assert step.step_number == 1
    assert step.trigger == "click"
    assert Path(step.image_path).exists()
    assert recorder.get_recorded_steps() == [step]
    mock_capture.capture_full.assert_awaited_once()


async def test_capture_multiple_steps(recorder: StepRecorder) -> None:
    """Three consecutive captures produce correctly numbered steps."""
    await recorder.start_recording("multi-test")
    s1 = await recorder.capture_step(trigger="click")
    s2 = await recorder.capture_step(trigger="enter")
    s3 = await recorder.capture_step(trigger="manual")

    assert s1.step_number == 1
    assert s2.step_number == 2
    assert s3.step_number == 3
    assert len(recorder.get_recorded_steps()) == 3


async def test_generate_playbook(recorder: StepRecorder, tmp_path: object) -> None:
    """generate_playbook creates playbook.md, config.yaml, and steps/."""
    from pathlib import Path

    output = Path(str(tmp_path)) / "playbook_out"
    await recorder.start_recording("gen-test")
    await recorder.capture_step(trigger="click", annotation="Click the button")
    await recorder.capture_step(trigger="enter", annotation="Press enter")
    recording = await recorder.stop_recording()

    result = await recorder.generate_playbook(recording, output)

    assert result == output
    assert (output / "playbook.md").exists()
    assert (output / "config.yaml").exists()
    assert (output / "steps").is_dir()

    playbook_text = (output / "playbook.md").read_text(encoding="utf-8")
    assert "gen-test" in playbook_text
    assert "Click the button" in playbook_text

    # steps/ should contain the screenshot PNGs
    step_files = list((output / "steps").iterdir())
    assert len(step_files) >= 2


async def test_not_recording_error(recorder: StepRecorder) -> None:
    """capture_step without start_recording raises RuntimeError."""
    with pytest.raises(RuntimeError, match="Not recording"):
        await recorder.capture_step()


async def test_annotation_saved(recorder: StepRecorder, tmp_path: object) -> None:
    """Capturing with an annotation creates a .md file alongside the PNG."""
    from pathlib import Path

    await recorder.start_recording("anno-test")
    step = await recorder.capture_step(trigger="manual", annotation="Important step")

    # The .md sidecar should exist next to the image
    image_path = Path(step.image_path)
    md_path = image_path.with_suffix(".md")
    assert md_path.exists()
    assert md_path.read_text(encoding="utf-8") == "Important step"
