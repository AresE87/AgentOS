# AgentOS Python SDK

Lightweight Python client for the AgentOS REST API.

## Installation

```bash
pip install agentos-sdk
```

Or install from source:
```bash
git clone https://github.com/AresEkb/AgentOS.git
cd AgentOS/sdk/python
pip install .
```

## Quick Start

```python
from agentos_sdk import AgentOS

agent = AgentOS(host="http://localhost:8080", api_key="aos_yourkey")
result = agent.send_task("check disk space")
print(result)
```

## Configuration

```python
agent = AgentOS(
    host="http://localhost:8080",   # AgentOS server address
    api_key="aos_yourkey"           # API key from Settings > API Keys
)
```

| Parameter | Type   | Default                  | Description              |
|-----------|--------|--------------------------|--------------------------|
| `host`    | `str`  | `http://localhost:8080`  | AgentOS server URL       |
| `api_key` | `str`  | `""`                     | Bearer token for auth    |

## Methods

### `agent.health() -> dict`
Check if AgentOS is running.

```python
status = agent.health()
# {"status": "ok", "version": "0.47.0"}
```

### `agent.status() -> dict`
Get current agent status including active tasks and system info.

```python
info = agent.status()
# {"state": "idle", "tasks_completed": 42, "uptime": 3600}
```

### `agent.send_task(text: str) -> dict`
Send a natural-language task to the agent.

```python
result = agent.send_task("list all running processes")
# {"task_id": "t_abc123", "status": "completed", "result": "..."}
```

### `agent.get_task(task_id: str) -> dict`
Check the result of a previously submitted task.

```python
task = agent.get_task("t_abc123")
# {"task_id": "t_abc123", "status": "completed", "result": "..."}
```

## Error Handling

```python
import requests

try:
    result = agent.send_task("check disk space")
except requests.ConnectionError:
    print("Cannot connect to AgentOS — is it running?")
except requests.HTTPError as e:
    print(f"API error: {e.response.status_code}")
```

## Examples

### Poll for Task Completion

```python
import time

result = agent.send_task("run full system scan")
task_id = result["task_id"]

while True:
    task = agent.get_task(task_id)
    if task["status"] in ("completed", "failed"):
        break
    time.sleep(2)

print(task["result"])
```

### Batch Tasks

```python
tasks = [
    "check disk space",
    "check CPU usage",
    "list running services",
]

results = [agent.send_task(t) for t in tasks]
for r in results:
    print(r["task_id"], r["status"])
```
