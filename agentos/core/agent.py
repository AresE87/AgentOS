"""Central agent pipeline that connects all components."""

from __future__ import annotations

import asyncio
import re
import time
from datetime import UTC, datetime
from typing import TYPE_CHECKING

from agentos.context.parser import ContextFolderParser
from agentos.executor.cli import CLIExecutor, CommandBlockedError, CommandTimeoutError
from agentos.gateway.classifier import BaseClassifier, RuleBasedClassifier
from agentos.types import (
    ContextFolder,
    LLMRequest,
    LLMResponse,
    TaskInput,
    TaskResult,
    TaskStatus,
)
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from pathlib import Path

    from agentos.gateway.gateway import LLMGateway
    from agentos.hierarchy.levels import AgentProfile
    from agentos.store.task_store import TaskStore

logger = get_logger("core")

DEFAULT_SYSTEM_PROMPT = """You are AgentOS, an AI assistant running locally on the user's PC.

You can:
- Answer questions using your knowledge
- Run shell commands on this machine when the user asks
- Help with code, analysis, writing, and general tasks

When the user asks you to do something on their computer, respond with the exact shell command to run inside a ```bash code block. Only suggest one command at a time.

When the user asks a general question, just answer it directly.

Be concise. This is a chat interface, not an essay.
Keep responses under 500 words unless the user asks for more detail.
"""


def extract_cli_command(llm_output: str) -> str | None:
    """Extract a CLI command from LLM output.

    Looks for code blocks marked as bash/shell/sh/zsh.
    Returns the command if exactly one code block found, else None.
    """
    pattern = r"```(?:bash|shell|sh|zsh)\n(.*?)```"
    matches = re.findall(pattern, llm_output, re.DOTALL)
    if len(matches) == 1:
        return matches[0].strip()
    return None


class AgentCore:
    """Central task processing pipeline."""

    def __init__(
        self,
        gateway: LLMGateway | None = None,
        classifier: BaseClassifier | None = None,
        executor: CLIExecutor | None = None,
        parser: ContextFolderParser | None = None,
        store: TaskStore | None = None,
        active_playbook: Path | None = None,
        max_concurrent_tasks: int = 5,
    ) -> None:
        self._gateway = gateway
        self._classifier = classifier or RuleBasedClassifier()
        self._executor = executor
        self._parser = parser or ContextFolderParser()
        self._store = store
        self._active_playbook_path = active_playbook
        self._active_context: ContextFolder | None = None
        self._semaphore = asyncio.Semaphore(max_concurrent_tasks)

    async def start(self) -> None:
        """Initialize all components."""
        if self._store:
            await self._store.initialize()

        # Parse active playbook if set
        if self._active_playbook_path and self._parser:
            try:
                self._active_context = await self._parser.parse(self._active_playbook_path)
                logger.info("Active playbook loaded: %s", self._active_context.config.name)
            except Exception:
                logger.warning("Failed to load playbook at %s", self._active_playbook_path)

        logger.info("AgentCore started")

    async def shutdown(self) -> None:
        """Gracefully shutdown all components."""
        if self._store:
            await self._store.close()
        logger.info("AgentCore shutdown")

    def set_active_playbook(self, path: Path | None) -> None:
        """Set or clear the active playbook path."""
        self._active_playbook_path = path
        self._active_context = None  # Will be loaded on next process()

    async def process(
        self,
        task_input: TaskInput,
        profile: AgentProfile | None = None,
        chain_context: list[str] | None = None,
    ) -> TaskResult:
        """Process a task. NEVER raises exceptions.

        Args:
            task_input: The task to process.
            profile: Optional agent profile that overrides system prompt and tier.
            chain_context: Optional list of outputs from dependency tasks to
                prepend to the prompt.
        """
        async with self._semaphore:
            return await self._process_internal(task_input, profile, chain_context)

    async def _process_internal(
        self,
        task_input: TaskInput,
        profile: AgentProfile | None = None,
        chain_context: list[str] | None = None,
    ) -> TaskResult:
        """Internal pipeline: create → classify → context → plan → execute → respond."""
        start_time = time.monotonic()
        task_id = task_input.task_id
        classification = None
        llm_response: LLMResponse | None = None

        try:
            # STEP 1: Create task in store
            if self._store:
                await self._store.create_task(task_input)

            # STEP 2: Classify
            classification = await self._classifier.classify(task_input)
            logger.info(
                "Task %s classified: %s/%d",
                task_id,
                classification.task_type.value,
                classification.complexity,
            )
            if self._store:
                await self._store.update_task_classification(task_id, classification)

            # STEP 3: Load context
            if profile:
                system_prompt = profile.system_prompt
                request_tier = profile.tier
            else:
                system_prompt = DEFAULT_SYSTEM_PROMPT
                request_tier = classification.tier
                if self._active_context:
                    system_prompt = self._active_context.instructions
                elif self._active_playbook_path and self._parser:
                    try:
                        self._active_context = await self._parser.parse(
                            self._active_playbook_path,
                        )
                        system_prompt = self._active_context.instructions
                    except Exception:
                        logger.warning("Failed to load playbook, using default")

            # Prepend dependency outputs when chaining agents
            prompt_text = task_input.text
            if chain_context:
                context_block = "\n---\n".join(chain_context)
                prompt_text = (
                    f"Previous task outputs:\n{context_block}\n\nCurrent task:\n{prompt_text}"
                )

            # STEP 4: Plan (LLM call)
            if self._store:
                await self._store.update_task_status(task_id, TaskStatus.RUNNING)

            output_text = ""
            if self._gateway:
                request = LLMRequest(
                    prompt=prompt_text,
                    tier=request_tier,
                    task_type=classification.task_type,
                    system_prompt=system_prompt,
                )
                llm_response = await self._gateway.complete(request)
                output_text = llm_response.content
            else:
                output_text = "No AI providers configured. Add at least one API key to .env"

            # STEP 5: Execute (if LLM suggests a CLI command)
            command = extract_cli_command(output_text) if llm_response else None
            if command and self._executor:
                logger.info("Executing CLI command for task %s", task_id)
                try:
                    exec_result = await self._executor.execute(command)
                    if self._store:
                        await self._store.save_execution(task_id, exec_result)

                    if exec_result.exit_code == 0:
                        output_text = (
                            exec_result.stdout or "Command completed successfully (no output)."
                        )
                    else:
                        output_text = (
                            f"Command failed (exit {exec_result.exit_code}):\n"
                            f"{exec_result.stderr or exec_result.stdout}"
                        )
                except CommandBlockedError as e:
                    output_text = f"Command blocked for safety: {e}"
                except CommandTimeoutError as e:
                    output_text = f"Command timed out: {e}"
                except Exception as e:
                    output_text = f"Command execution failed: {e}"

            # STEP 6: Respond
            elapsed = (time.monotonic() - start_time) * 1000
            if self._store:
                await self._store.complete_task(task_id, output_text, llm_response)

            return TaskResult(
                task_id=task_id,
                input_text=task_input.text,
                source=task_input.source,
                status=TaskStatus.COMPLETED,
                classification=classification,
                model_used=llm_response.model if llm_response else None,
                provider=llm_response.provider if llm_response else None,
                tokens_in=llm_response.tokens_in if llm_response else 0,
                tokens_out=llm_response.tokens_out if llm_response else 0,
                cost_estimate=llm_response.cost_estimate if llm_response else 0.0,
                output_text=output_text,
                created_at=task_input.created_at,
                completed_at=datetime.now(UTC),
                duration_ms=elapsed,
            )

        except Exception as e:
            # CATCH: Never raise — always return a result
            elapsed = (time.monotonic() - start_time) * 1000
            logger.exception("Task %s failed", task_id)

            error_msg = str(e)
            if self._store:
                try:
                    await self._store.fail_task(task_id, error_msg)
                except Exception:
                    logger.warning("Failed to persist task failure")

            return TaskResult(
                task_id=task_id,
                input_text=task_input.text,
                source=task_input.source,
                status=TaskStatus.FAILED,
                classification=classification,
                error_message=error_msg,
                created_at=task_input.created_at,
                completed_at=datetime.now(UTC),
                duration_ms=elapsed,
            )
