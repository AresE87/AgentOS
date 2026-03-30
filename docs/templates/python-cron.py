#!/usr/bin/env python3
"""
AgentOS Scheduled Task Runner

Calls the AgentOS API on a schedule using Python's sched module.
No external dependencies beyond `requests`.

Usage:
    python python-cron.py

Configuration:
    Set AGENTOS_HOST and AGENTOS_API_KEY environment variables,
    or edit the defaults below.
"""

import os
import time
import sched
import logging
import requests

# ── Configuration ──────────────────────────────────────────────
HOST = os.getenv("AGENTOS_HOST", "http://localhost:8080")
API_KEY = os.getenv("AGENTOS_API_KEY", "aos_yourkey")
INTERVAL_SECONDS = 3600  # Run every hour (adjust as needed)

TASKS = [
    "run system health check",
    "check disk space and alert if below 10%",
]
# ───────────────────────────────────────────────────────────────

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
log = logging.getLogger("agentos-cron")

session = requests.Session()
session.headers.update({
    "Content-Type": "application/json",
    "Authorization": f"Bearer {API_KEY}",
})

scheduler = sched.scheduler(time.time, time.sleep)


def send_task(text: str) -> dict:
    """Send a single task to AgentOS and return the response."""
    try:
        resp = session.post(f"{HOST}/v1/message", json={"text": text}, timeout=30)
        resp.raise_for_status()
        data = resp.json()
        log.info("Task sent: %s -> %s", text[:50], data.get("task_id", "?"))
        return data
    except requests.RequestException as exc:
        log.error("Failed to send task '%s': %s", text[:50], exc)
        return {"error": str(exc)}


def run_scheduled_tasks():
    """Execute all configured tasks and reschedule."""
    log.info("Running %d scheduled task(s)...", len(TASKS))
    for task in TASKS:
        send_task(task)

    # Reschedule
    scheduler.enter(INTERVAL_SECONDS, 1, run_scheduled_tasks)


def main():
    log.info("AgentOS Cron Runner started. Interval: %ds", INTERVAL_SECONDS)
    log.info("Host: %s", HOST)

    # Run immediately, then on schedule
    scheduler.enter(0, 1, run_scheduled_tasks)
    try:
        scheduler.run()
    except KeyboardInterrupt:
        log.info("Stopped by user.")


if __name__ == "__main__":
    main()
