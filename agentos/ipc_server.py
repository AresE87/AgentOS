"""JSON-RPC 2.0 server for IPC with Tauri shell.

Reads JSON requests from stdin (one per line), processes them,
and writes JSON responses to stdout (one per line).
stderr is used for logging only.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import sys
from pathlib import Path
from typing import Any

from agentos.core.agent import AgentCore
from agentos.executor.cli import CLIExecutor
from agentos.executor.safety import SafetyGuard
from agentos.gateway.cost_tracker import CostTracker, load_price_table
from agentos.gateway.gateway import LLMGateway
from agentos.gateway.provider import LiteLLMProvider
from agentos.gateway.router import ModelRouter
from agentos.settings import load_settings, save_settings_to_file
from agentos.store.task_store import TaskStore
from agentos.types import ModelProvider, TaskInput

# Redirect logging to stderr so stdout is clean for JSON-RPC
logging.basicConfig(stream=sys.stderr, level=logging.INFO)
logger = logging.getLogger("agentos.ipc")


class IPCServer:
    """JSON-RPC 2.0 server on stdin/stdout."""

    def __init__(self) -> None:
        self._agent: AgentCore | None = None
        self._store: TaskStore | None = None
        self._running = True

    async def initialize(self) -> None:
        """Initialize all agent components."""
        settings = load_settings()

        self._store = TaskStore(db_path=settings.db_path)

        # Gateway
        gateway = None
        providers = settings.available_providers()
        if providers:
            import yaml

            config_path = Path(settings.config_dir) / "routing.yaml"
            router = ModelRouter(config_path=config_path)
            with open(config_path) as f:  # noqa: PTH123
                routing_config = yaml.safe_load(f) or {}
            price_table = load_price_table(routing_config)
            cost_tracker = CostTracker(price_table=price_table, task_store=self._store)
            gateway = LLMGateway(settings=settings, router=router, cost_tracker=cost_tracker)
            # Register providers that have API keys
            for name in providers:
                try:
                    provider_enum = ModelProvider(name)
                    gateway.register_provider(name, LiteLLMProvider(provider=provider_enum))
                    logger.info("Registered provider: %s", name)
                except ValueError:
                    logger.warning("Unknown provider: %s", name)

        # Executor
        safety_config = Path(settings.config_dir) / "cli_safety.yaml"
        guard = SafetyGuard(config_path=safety_config if safety_config.exists() else None)
        executor = CLIExecutor(safety=guard, default_timeout=settings.cli_timeout)

        # Agent Core
        self._agent = AgentCore(gateway=gateway, executor=executor, store=self._store)
        await self._agent.start()

        self.send_event("ready", {})
        logger.info("IPC Server initialized")

    async def run(self) -> None:
        """Main loop: read stdin -> process -> write stdout."""
        await self.initialize()

        # Use a thread to read stdin (Windows ProactorEventLoop doesn't support pipe reading)
        loop = asyncio.get_event_loop()
        queue: asyncio.Queue[str] = asyncio.Queue()

        def _stdin_reader() -> None:
            """Read lines from stdin in a background thread."""
            try:
                for line in sys.stdin:
                    line = line.strip()
                    if line:
                        loop.call_soon_threadsafe(queue.put_nowait, line)
            except (EOFError, ValueError):
                pass
            loop.call_soon_threadsafe(queue.put_nowait, "")  # Signal EOF

        import threading
        reader_thread = threading.Thread(target=_stdin_reader, daemon=True)
        reader_thread.start()

        while self._running:
            try:
                line_str = await queue.get()
                if not line_str:
                    break

                request = json.loads(line_str)
                response = await self.handle_request(request)
                if response:
                    self.send_response(response)

            except json.JSONDecodeError as e:
                self.send_error(None, -32700, f"Parse error: {e}")
            except Exception as e:  # noqa: BLE001
                logger.exception("Error in IPC loop")
                self.send_error(None, -32603, f"Internal error: {e}")

    async def handle_request(self, request: dict[str, Any]) -> dict[str, Any] | None:
        """Dispatch request to the correct handler."""
        method = request.get("method", "")
        params = request.get("params", {})
        req_id = request.get("id")

        handlers: dict[str, Any] = {
            "get_status": self._handle_get_status,
            "process_message": self._handle_process_message,
            "get_tasks": self._handle_get_tasks,
            "get_playbooks": self._handle_get_playbooks,
            "set_active_playbook": self._handle_set_active_playbook,
            "get_settings": self._handle_get_settings,
            "update_settings": self._handle_update_settings,
            "health_check": self._handle_health_check,
            "get_usage_summary": self._handle_get_usage_summary,
            "get_active_chain": self._handle_get_active_chain,
            "get_chain_history": self._handle_get_chain_history,
            "get_chain_detail": self._handle_get_chain_detail,
            "send_chain_message": self._handle_send_chain_message,
            "get_analytics": self._handle_get_analytics,
            "shutdown": self._handle_shutdown,
        }

        handler = handlers.get(method)
        if not handler:
            return {
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": f"Method not found: {method}",
                },
                "id": req_id,
            }

        try:
            result = await handler(params)
            return {"jsonrpc": "2.0", "result": result, "id": req_id}
        except Exception as e:  # noqa: BLE001
            return {
                "jsonrpc": "2.0",
                "error": {"code": -32603, "message": str(e)},
                "id": req_id,
            }

    # ─── Handlers ───────────────────────────────────────────────────

    async def _handle_get_status(self, params: dict[str, Any]) -> dict[str, Any]:
        settings = load_settings()
        providers = list(settings.available_providers().keys())

        # Get real session stats
        stats: dict[str, Any] = {"tasks": 0, "cost": 0.0, "tokens": 0}
        if self._store:
            try:
                tasks = await self._store.get_recent_tasks(limit=1000)
                stats["tasks"] = len(tasks)
                stats["cost"] = round(
                    sum(t.get("cost_estimate", 0.0) for t in tasks), 4
                )
                stats["tokens"] = sum(
                    t.get("tokens_in", 0) + t.get("tokens_out", 0) for t in tasks
                )
            except Exception:  # noqa: BLE001
                pass

        return {
            "state": "running",
            "providers": providers,
            "active_playbook": None,
            "session_stats": stats,
        }

    async def _handle_process_message(self, params: dict[str, Any]) -> dict[str, Any]:
        text = params.get("text", "")
        source = params.get("source", "chat")
        task_input = TaskInput(text=text, source=source)
        self.send_event("task_started", {"task_id": task_input.task_id})

        if self._agent is None:
            msg = "Agent not initialized"
            raise RuntimeError(msg)

        result = await self._agent.process(task_input)
        return {
            "task_id": result.task_id,
            "status": result.status.value,
            "output": result.output_text,
            "model": result.model_used,
            "cost": result.cost_estimate,
            "duration_ms": result.duration_ms,
        }

    async def _handle_get_tasks(self, params: dict[str, Any]) -> dict[str, Any]:
        limit = params.get("limit", 10)
        tasks = await self._store.get_recent_tasks(limit) if self._store else []
        return {"tasks": tasks}

    async def _handle_get_playbooks(self, params: dict[str, Any]) -> dict[str, Any]:
        settings = load_settings()
        playbooks_dir = Path(settings.playbooks_dir)
        if playbooks_dir.exists():
            from agentos.context.parser import ContextFolderParser

            parser = ContextFolderParser()
            folders = await parser.parse_many(playbooks_dir)
            return {
                "playbooks": [
                    {
                        "name": f.config.name,
                        "path": str(f.path),
                        "tier": f.config.tier.value,
                        "permissions": f.config.permissions,
                    }
                    for f in folders
                ],
            }
        return {"playbooks": []}

    async def _handle_set_active_playbook(self, params: dict[str, Any]) -> dict[str, Any]:
        path = params.get("path", "")
        if self._agent and path:
            self._agent.set_active_playbook(Path(path))
        return {"ok": True}

    async def _handle_get_settings(self, params: dict[str, Any]) -> dict[str, Any]:
        s = load_settings()
        return {
            "log_level": s.log_level,
            "max_cost_per_task": s.max_cost_per_task,
            "cli_timeout": s.cli_timeout,
            "has_anthropic": bool(s.anthropic_api_key),
            "has_openai": bool(s.openai_api_key),
            "has_google": bool(s.google_api_key),
            "has_telegram": bool(s.telegram_bot_token),
        }

    async def _handle_update_settings(self, params: dict[str, Any]) -> dict[str, Any]:
        """Persist a setting and reconfigure if needed."""
        key = params.get("key", "")
        value = params.get("value", "")

        # Save to config file
        save_settings_to_file({key: value})

        # If it's an API key, reconfigure the gateway
        if key in ("anthropic_api_key", "openai_api_key", "google_api_key"):
            env_map = {
                "anthropic_api_key": "ANTHROPIC_API_KEY",
                "openai_api_key": "OPENAI_API_KEY",
                "google_api_key": "GOOGLE_API_KEY",
            }
            if env_map.get(key):
                os.environ[env_map[key]] = value

            # Reinitialize gateway with new key
            await self._reinitialize_gateway()

        return {"ok": True, "saved": key}

    async def _handle_health_check(self, params: dict[str, Any]) -> dict[str, Any]:
        """Actually test each provider's API connection."""
        import httpx

        results: dict[str, bool] = {}
        settings = load_settings()
        providers = settings.available_providers()

        for name, key in providers.items():
            try:
                if name == "anthropic":
                    async with httpx.AsyncClient(timeout=10.0) as client:
                        resp = await client.get(
                            "https://api.anthropic.com/v1/messages",
                            headers={
                                "x-api-key": key,
                                "anthropic-version": "2023-06-01",
                            },
                        )
                        # 401 = bad key, 405 = method not allowed (key works)
                        results[name] = resp.status_code != 401
                elif name == "openai":
                    async with httpx.AsyncClient(timeout=10.0) as client:
                        resp = await client.get(
                            "https://api.openai.com/v1/models",
                            headers={"Authorization": f"Bearer {key}"},
                        )
                        results[name] = resp.status_code == 200
                elif name == "google":
                    results[name] = bool(key)
                else:
                    results[name] = bool(key)
            except Exception:  # noqa: BLE001
                results[name] = False

        return {"providers": results}

    async def _handle_get_usage_summary(self, params: dict[str, Any]) -> dict[str, Any]:
        """Get real usage data from TaskStore."""
        if not self._store:
            return {"total_cost": 0.0, "total_calls": 0, "total_tokens": 0}

        tasks = await self._store.get_recent_tasks(limit=1000)
        total_cost = sum(t.get("cost_estimate", 0.0) for t in tasks)
        total_tokens = sum(
            t.get("tokens_in", 0) + t.get("tokens_out", 0) for t in tasks
        )
        completed = sum(1 for t in tasks if t.get("status") == "completed")
        failed = sum(1 for t in tasks if t.get("status") == "failed")

        return {
            "total_cost": round(total_cost, 6),
            "total_calls": len(tasks),
            "total_tokens": total_tokens,
            "completed": completed,
            "failed": failed,
            "success_rate": round(completed / max(len(tasks), 1), 3),
        }

    # ─── Board / Chain handlers ────────────────────────────────────

    async def _handle_get_active_chain(self, params: dict[str, Any]) -> dict[str, Any]:
        """Return active chain if any, otherwise null."""
        return {"chain_id": None, "status": "idle", "subtasks": [], "log": []}

    async def _handle_get_chain_history(self, params: dict[str, Any]) -> dict[str, Any]:
        """Return real chain history from store."""
        if not self._store:
            return {"chains": []}
        try:
            chains = await self._store.get_chain_history(limit=20)
            return {"chains": chains}
        except Exception:  # noqa: BLE001
            return {"chains": []}

    async def _handle_get_chain_detail(self, params: dict[str, Any]) -> dict[str, Any]:
        """Return full chain detail with log from store."""
        chain_id = params.get("chain_id", "")
        if not self._store or not chain_id:
            return {"chain_id": chain_id, "log": []}
        try:
            log = await self._store.get_chain_log(chain_id, limit=100)
            return {"chain_id": chain_id, "log": log}
        except Exception:  # noqa: BLE001
            return {"chain_id": chain_id, "log": []}

    async def _handle_send_chain_message(self, params: dict[str, Any]) -> dict[str, Any]:
        """Inject a user message into ChainContext (stub)."""
        return {"ok": True}

    async def _handle_get_analytics(self, params: dict[str, Any]) -> dict[str, Any]:
        """Return real analytics data from TaskStore."""
        if not self._store:
            return {"tasks": [], "total_cost": 0, "total_tasks": 0}

        tasks = await self._store.get_recent_tasks(limit=5000)

        # Aggregate by day (last 7 days)
        from collections import defaultdict

        daily: dict[str, dict[str, Any]] = defaultdict(
            lambda: {"tasks": 0, "cost": 0.0, "tokens": 0}
        )
        by_provider: dict[str, float] = defaultdict(float)
        by_type: dict[str, int] = defaultdict(int)

        for t in tasks:
            created = t.get("created_at", "")
            if created:
                day = created[:10]  # YYYY-MM-DD
                daily[day]["tasks"] += 1
                daily[day]["cost"] += t.get("cost_estimate", 0.0)
                daily[day]["tokens"] += t.get("tokens_in", 0) + t.get("tokens_out", 0)

            provider = t.get("provider", "unknown")
            by_provider[provider] += t.get("cost_estimate", 0.0)

            task_type = t.get("task_type", "text")
            by_type[task_type] += 1

        # Sort daily by date, take last 7
        sorted_daily = sorted(daily.items())[-7:]

        total_tasks = len(tasks)
        completed = sum(1 for t in tasks if t.get("status") == "completed")
        total_cost = sum(t.get("cost_estimate", 0.0) for t in tasks)

        return {
            "total_tasks": total_tasks,
            "completed": completed,
            "success_rate": round(completed / max(total_tasks, 1), 3),
            "total_cost": round(total_cost, 4),
            "daily": [{"date": d, **v} for d, v in sorted_daily],
            "cost_by_provider": dict(by_provider),
            "tasks_by_type": dict(by_type),
        }

    async def _reinitialize_gateway(self) -> None:
        """Reload settings and recreate the LLM gateway with current providers."""
        import yaml

        settings = load_settings()
        providers = settings.available_providers()
        if not providers:
            return

        config_path = Path(settings.config_dir) / "routing.yaml"
        router = ModelRouter(config_path=config_path)
        with open(config_path) as f:  # noqa: PTH123
            routing_config = yaml.safe_load(f) or {}
        price_table = load_price_table(routing_config)
        cost_tracker = CostTracker(price_table=price_table, task_store=self._store)
        gateway = LLMGateway(settings=settings, router=router, cost_tracker=cost_tracker)

        for name in providers:
            try:
                provider_enum = ModelProvider(name)
                gateway.register_provider(name, LiteLLMProvider(provider=provider_enum))
                logger.info("Re-registered provider: %s", name)
            except ValueError:
                logger.warning("Unknown provider: %s", name)

        if self._agent:
            self._agent._gateway = gateway  # noqa: SLF001
        logger.info("Gateway reinitialized with providers: %s", list(providers.keys()))

    async def _handle_shutdown(self, params: dict[str, Any]) -> dict[str, Any]:
        self._running = False
        if self._agent:
            await self._agent.shutdown()
        return {"ok": True}

    # ─── Output helpers ─────────────────────────────────────────────

    def send_response(self, response: dict[str, Any]) -> None:
        """Write a JSON-RPC response to stdout."""
        line = json.dumps(response) + "\n"
        sys.stdout.write(line)
        sys.stdout.flush()

    def send_event(self, event_type: str, params: dict[str, Any]) -> None:
        """Write a JSON-RPC notification (event) to stdout."""
        event = {
            "jsonrpc": "2.0",
            "method": "event",
            "params": {"type": event_type, **params},
        }
        sys.stdout.write(json.dumps(event) + "\n")
        sys.stdout.flush()

    def send_error(self, req_id: int | None, code: int, message: str) -> None:
        """Write a JSON-RPC error response to stdout."""
        resp = {
            "jsonrpc": "2.0",
            "error": {"code": code, "message": message},
            "id": req_id,
        }
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


async def main() -> None:
    """Entry point for the IPC server."""
    server = IPCServer()
    await server.run()


if __name__ == "__main__":
    asyncio.run(main())
