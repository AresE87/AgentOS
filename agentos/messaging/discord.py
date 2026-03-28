"""Discord bot adapter."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

from agentos.messaging.base import BaseMessagingAdapter
from agentos.types import TaskInput, TaskResult, TaskStatus
from agentos.utils.logging import get_logger

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

logger = get_logger("messaging.discord")


class DiscordAdapter(BaseMessagingAdapter):
    """Discord bot adapter using discord.py."""

    def __init__(
        self,
        token: str,
        on_message: Callable[[TaskInput], Awaitable[TaskResult]],
    ) -> None:
        super().__init__(on_message)
        self._token = token
        self._bot = None  # type: ignore[assignment]
        self._running = False

    # -- Lifecycle -------------------------------------------------------------

    async def start(self) -> None:
        """Start the Discord bot."""
        try:
            import discord  # noqa: TCH004
            from discord.ext import commands
        except ImportError:
            logger.error("discord.py not installed. Run: pip install discord.py")
            return

        intents = discord.Intents.default()
        intents.message_content = True
        self._bot = commands.Bot(command_prefix="!", intents=intents)

        bot = self._bot

        @bot.event
        async def on_ready() -> None:
            logger.info("Discord bot connected as %s", bot.user)

        @bot.event
        async def on_message(message: discord.Message) -> None:
            if message.author == bot.user:
                return
            # Let the commands extension handle prefixed commands first.
            if message.content.startswith("!"):
                await bot.process_commands(message)
                return
            # Everything else is treated as a task.
            await self._handle_message(message)

        @bot.command(name="status")  # type: ignore[arg-type]
        async def cmd_status(ctx: commands.Context) -> None:  # type: ignore[type-arg]
            await ctx.send("AgentOS is running and ready for tasks.")

        @bot.command(name="history")  # type: ignore[arg-type]
        async def cmd_history(ctx: commands.Context) -> None:  # type: ignore[type-arg]
            await ctx.send("No task history available yet.")

        @bot.command(name="help_agent")  # type: ignore[arg-type]
        async def cmd_help(ctx: commands.Context) -> None:  # type: ignore[type-arg]
            embed = discord.Embed(
                title="AgentOS Help",
                description="Send me any message and I'll help!",
                color=0x8B5CF6,
            )
            embed.add_field(name="!status", value="Check agent health", inline=True)
            embed.add_field(name="!history", value="Recent tasks", inline=True)
            await ctx.send(embed=embed)

        self._running = True
        asyncio.create_task(bot.start(self._token))
        logger.info("Discord adapter started")

    async def stop(self) -> None:
        """Stop the Discord bot."""
        self._running = False
        if self._bot:
            await self._bot.close()
        logger.info("Discord adapter stopped")

    # -- Sending ---------------------------------------------------------------

    async def send_message(self, chat_id: str, text: str) -> None:
        """Send a message to a Discord channel by ID."""
        if not self._bot:
            return
        try:
            channel = self._bot.get_channel(int(chat_id))
            if channel:
                for chunk in self._split_message(text):
                    await channel.send(chunk)
        except Exception:
            logger.exception("Failed to send Discord message to %s", chat_id)

    # -- Internal --------------------------------------------------------------

    async def _handle_message(self, message) -> None:  # type: ignore[no-untyped-def]
        text = message.content.strip()
        if not text:
            return

        chat_id = str(message.channel.id)
        task_input = TaskInput(text=text, source="discord", chat_id=chat_id)

        async with message.channel.typing():
            try:
                result = await self._on_message(task_input)
                response = self._format_result(result)
                for chunk in self._split_message(response):
                    await message.channel.send(chunk)
            except Exception:
                logger.exception("Error processing Discord message")
                await message.channel.send("An error occurred processing your request.")

    @staticmethod
    def _format_result(result: TaskResult) -> str:
        """Format a *TaskResult* for Discord display (markdown)."""
        if result.status == TaskStatus.FAILED:
            return f"**Error:** {result.error_message or 'Unknown error'}"
        parts: list[str] = []
        if result.output_text:
            parts.append(result.output_text)
        if result.model_used:
            parts.append(f"\n*Model: {result.model_used} · Cost: ${result.cost_estimate:.4f}*")
        return "\n".join(parts) if parts else "Done."

    @staticmethod
    def _split_message(text: str, max_length: int = 2000) -> list[str]:
        """Split *text* into Discord-safe chunks (limit 2000 chars)."""
        if len(text) <= max_length:
            return [text]
        chunks: list[str] = []
        while text:
            if len(text) <= max_length:
                chunks.append(text)
                break
            split = text.rfind("\n", 0, max_length)
            if split == -1:
                split = max_length
            chunks.append(text[:split])
            text = text[split:].lstrip()
        return chunks
