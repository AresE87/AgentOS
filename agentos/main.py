"""AgentOS entry point.

Usage: python -m agentos.main
"""

from __future__ import annotations

import asyncio
import contextlib
import signal
from pathlib import Path

import yaml

from agentos.core.agent import AgentCore
from agentos.executor.cli import CLIExecutor
from agentos.executor.safety import SafetyGuard
from agentos.gateway.cost_tracker import CostTracker, load_price_table
from agentos.gateway.gateway import LLMGateway
from agentos.gateway.provider import LiteLLMProvider
from agentos.gateway.router import ModelRouter
from agentos.types import ModelProvider
from agentos.messaging.telegram import TelegramAdapter
from agentos.settings import load_settings
from agentos.store.task_store import TaskStore
from agentos.utils.logging import get_logger, setup_logging

logger = get_logger("main")


async def main() -> None:
    """Start the AgentOS agent."""
    settings = load_settings()
    setup_logging(settings.log_level)

    logger.info("AgentOS v0.1.0 starting...")
    logger.info("Settings: %s", settings)

    # Initialize components
    store = TaskStore(db_path=settings.db_path)

    # LLM Gateway (only if providers available)
    gateway = None
    providers = settings.available_providers()
    if providers:
        logger.info("Available providers: %s", list(providers.keys()))
        config_path = Path(settings.config_dir) / "routing.yaml"
        router = ModelRouter(config_path=config_path)

        # Build price table from routing.yaml for cost tracking
        with open(config_path, encoding="utf-8") as fh:
            routing_config = yaml.safe_load(fh) or {}
        price_table = load_price_table(routing_config)

        cost_tracker = CostTracker(price_table=price_table, task_store=store)
        gateway = LLMGateway(
            settings=settings,
            router=router,
            cost_tracker=cost_tracker,
        )
        # Register providers
        for name in providers:
            try:
                gateway.register_provider(name, LiteLLMProvider(provider=ModelProvider(name)))
            except ValueError:
                pass
    else:
        logger.warning("No AI providers configured. Add API keys to .env")

    # CLI Executor with SafetyGuard
    safety_config = Path(settings.config_dir) / "cli_safety.yaml"
    guard = SafetyGuard(
        config_path=safety_config if safety_config.exists() else None,
    )
    executor = CLIExecutor(safety=guard, default_timeout=settings.cli_timeout)

    # Agent Core
    agent = AgentCore(
        gateway=gateway,
        executor=executor,
        store=store,
    )
    await agent.start()

    # Telegram Bot (only if token configured)
    telegram: TelegramAdapter | None = None
    if settings.telegram_bot_token:
        telegram = TelegramAdapter(
            token=settings.telegram_bot_token,
            process_callback=agent.process,
        )
        await telegram.start()
        logger.info("Telegram bot started")
    else:
        logger.info("No Telegram token configured — skipping bot")

    logger.info("AgentOS ready. Press Ctrl+C to stop.")

    # Wait for shutdown signal
    stop_event = asyncio.Event()

    def _signal_handler() -> None:
        logger.info("Shutdown signal received...")
        stop_event.set()

    loop = asyncio.get_event_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        with contextlib.suppress(NotImplementedError):
            loop.add_signal_handler(sig, _signal_handler)

    with contextlib.suppress(KeyboardInterrupt):
        await stop_event.wait()

    # Graceful shutdown
    if telegram:
        await telegram.stop()
    await agent.shutdown()
    logger.info("AgentOS stopped.")


if __name__ == "__main__":
    asyncio.run(main())
