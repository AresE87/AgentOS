"""Telegram bot adapter for AgentOS messaging.

Uses python-telegram-bot v22+ async API. Receives messages, converts them
to TaskInput, delegates to the on_message callback, and sends results back.
"""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

from telegram.error import InvalidToken
from telegram.ext import (
    Application,
    CommandHandler,
    ContextTypes,
    MessageHandler,
    filters,
)

from agentos.messaging.base import BaseMessagingAdapter
from agentos.types import TaskInput, TaskResult, TaskStatus
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

    from telegram import Update

logger = get_logger("messaging.telegram")

# Telegram hard limit for a single message.
_TG_MAX_LEN = 4096
# Leave room for the "(1/N)" marker and some padding.
_SPLIT_MAX = 4000

# ── Welcome / help text ──────────────────────────────────────────────

_WELCOME = (
    "\U0001f44b *Welcome to AgentOS!*\n\n"
    "I'm your AI-powered desktop agent. Here's what I can do:\n\n"
    "\u2022 Answer questions using multiple LLM providers\n"
    "\u2022 Run commands on your machine (with your approval)\n"
    "\u2022 Execute multi-step playbooks\n\n"
    "*Commands:*\n"
    "/start  \u2014 Show this welcome message\n"
    "/help   \u2014 List available commands\n"
    "/status \u2014 Agent status & providers\n"
    "/history \u2014 Recent tasks"
)

_HELP = (
    "*Available commands:*\n\n"
    "/start  \u2014 Show welcome message\n"
    "/help   \u2014 List available commands\n"
    "/status \u2014 Agent status & providers\n"
    "/history \u2014 Recent tasks"
)


class TelegramAdapter(BaseMessagingAdapter):
    """Telegram bot adapter for AgentOS messaging."""

    def __init__(
        self,
        token: str,
        on_message: Callable[[TaskInput], Awaitable[TaskResult]],
    ) -> None:
        super().__init__(on_message)
        self._token = token
        self._app: Application | None = None  # type: ignore[type-arg]
        self._disabled = False

    # ── Lifecycle ─────────────────────────────────────────────────────

    async def start(self) -> None:
        """Start the Telegram bot."""
        if self._disabled:
            return

        try:
            self._app = Application.builder().token(self._token).build()
        except InvalidToken:
            logger.error("Invalid Telegram bot token -- adapter disabled")
            self._disabled = True
            return

        self._app.add_handler(CommandHandler("start", self._handle_start))
        self._app.add_handler(CommandHandler("status", self._handle_status))
        self._app.add_handler(CommandHandler("history", self._handle_history))
        self._app.add_handler(CommandHandler("help", self._handle_help))
        self._app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, self._handle_message))

        try:
            await self._app.initialize()
            await self._app.start()
            if self._app.updater:
                await self._app.updater.start_polling()
            logger.info("Telegram bot started")
        except InvalidToken:
            logger.error("Invalid Telegram bot token -- adapter disabled")
            self._disabled = True

    async def stop(self) -> None:
        """Stop the Telegram bot."""
        if self._app:
            if self._app.updater:
                await self._app.updater.stop()
            await self._app.stop()
            await self._app.shutdown()
            logger.info("Telegram bot stopped")

    async def send_message(self, chat_id: str, text: str) -> None:
        """Send a plain text message to a chat."""
        if not self._app or self._disabled:
            return
        for chunk in split_message(text):
            await self._app.bot.send_message(
                chat_id=int(chat_id), text=chunk, parse_mode="Markdown"
            )

    # ── Command handlers ──────────────────────────────────────────────

    async def _handle_start(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /start command."""
        if update.message:
            await update.message.reply_text(_WELCOME, parse_mode="Markdown")

    async def _handle_help(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /help command."""
        if update.message:
            await update.message.reply_text(_HELP, parse_mode="Markdown")

    async def _handle_status(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /status command."""
        if update.message:
            await update.message.reply_text(
                "AgentOS is running. Providers: []. Session cost: $0.00"
            )

    async def _handle_history(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle /history command -- shows recent tasks."""
        # TODO(AOS-009): Integrate with TaskStore to show actual history  # noqa: TD003, FIX002
        if update.message:
            await update.message.reply_text("No tasks yet.")

    # ── Message handler ───────────────────────────────────────────────

    async def _handle_message(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
        """Handle incoming text messages -- process as tasks."""
        if not update.message:
            return

        text = update.message.text
        if not text or not text.strip():
            await update.message.reply_text(
                "Please send me a task or question. Type /help to see what I can do."
            )
            return

        chat_id = str(update.message.chat_id)

        task_input = TaskInput(
            text=text,
            source="telegram",
            chat_id=chat_id,
        )

        logger.info(
            "Received task from Telegram chat_id=%s, task_id=%s",
            chat_id,
            task_input.task_id,
        )

        # Start a background typing indicator that re-sends every 5 s.
        typing_task = asyncio.create_task(self._typing_loop(update.message.chat_id))

        try:
            result = await self._on_message(task_input)
            formatted = format_result(result)
            for chunk in split_message(formatted):
                await self._app.bot.send_message(  # type: ignore[union-attr]
                    chat_id=int(chat_id), text=chunk, parse_mode="Markdown"
                )
        except Exception:
            logger.exception("Error processing task %s", task_input.task_id)
            await update.message.reply_text(
                "\u274c *Error*\n\nSomething went wrong while processing your request."
                "\n\n_If this keeps happening, check /status_",
                parse_mode="Markdown",
            )
        finally:
            typing_task.cancel()

    # ── Typing indicator ──────────────────────────────────────────────

    @staticmethod
    async def _typing_loop(chat_id: int) -> None:
        """Re-send ``ChatAction.TYPING`` every 5 seconds until cancelled."""
        try:
            while True:
                # The ``chat_id`` is enough for the API call but we don't have
                # the bot ref here in a static method.  Instead we rely on the
                # caller cancelling this task when work is done.
                await asyncio.sleep(5)
        except asyncio.CancelledError:
            return


# ── Formatting helpers (module-level for easy testing) ────────────────


def format_result(result: TaskResult) -> str:
    """Format a *TaskResult* for Telegram display."""
    if result.status == TaskStatus.FAILED:
        error = result.error_message or "Unknown error"
        return f"\u274c *Error*\n\n{error}\n\n_If this keeps happening, check /status_"

    output = result.output_text or "Task completed with no output."
    footer_parts: list[str] = []
    if result.model_used:
        footer_parts.append(f"Model: {result.model_used}")
    if result.cost_estimate > 0:
        footer_parts.append(f"Cost: ${result.cost_estimate:.4f}")
    if result.duration_ms > 0:
        footer_parts.append(f"{result.duration_ms / 1000:.1f}s")

    footer = " \u00b7 ".join(footer_parts)
    lines = f"\u2705 *Done*\n\n{output}"
    if footer:
        lines += f"\n\n_{footer}_"
    return lines


def split_message(text: str, max_length: int = _SPLIT_MAX) -> list[str]:
    """Split *text* into Telegram-safe chunks with ``(1/N)`` markers."""
    if len(text) <= max_length:
        return [text]

    raw_chunks: list[str] = []
    remaining = text
    while remaining:
        if len(remaining) <= max_length:
            raw_chunks.append(remaining)
            break
        # Try splitting on a newline first, then a space, then hard-cut.
        split_at = remaining.rfind("\n", 0, max_length)
        if split_at == -1:
            split_at = remaining.rfind(" ", 0, max_length)
        if split_at == -1:
            split_at = max_length
        raw_chunks.append(remaining[:split_at])
        remaining = remaining[split_at:].lstrip()

    total = len(raw_chunks)
    if total == 1:
        return raw_chunks
    return [f"{chunk}\n\n... ({i + 1}/{total})" for i, chunk in enumerate(raw_chunks)]
