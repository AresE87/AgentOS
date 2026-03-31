# AgentOS Python SDK Reference Client

This Python client mirrors the real local API contract exposed by AgentOS.

```python
import requests


class AgentOS:
    def __init__(self, host="http://localhost:8080", api_key=""):
        self.host = host.rstrip("/")
        self.api_key = api_key

    def _request(self, method, path, payload=None):
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"

        response = requests.request(
            method,
            f"{self.host}{path}",
            headers=headers,
            json=payload,
            timeout=30,
        )

        try:
            data = response.json()
        except ValueError:
            data = {}

        if not response.ok:
            message = data.get("message", f"HTTP {response.status_code}")
            error = requests.HTTPError(message, response=response)
            error.agentos_code = data.get("error", "http_error")
            raise error
        return data

    def health(self):
        return self._request("GET", "/health")

    def status(self):
        return self._request("GET", "/v1/status")

    def send_task(self, text):
        return self._request("POST", "/v1/message", {"text": text})

    def list_tasks(self, limit=20, status=None):
        suffix = f"/v1/tasks?limit={limit}"
        if status:
            suffix += f"&status={status}"
        return self._request("GET", suffix)

    def get_task(self, task_id):
        return self._request("GET", f"/v1/task/{task_id}")
```

## Example

```python
client = AgentOS(api_key="aos_your_key_here")
queued = client.send_task("collect recent error counts")
print(queued)

task = client.get_task(queued["task_id"])
print(task["status"], task.get("result"))
```

## Real response shapes

`client.status()`:

```json
{
  "status": "running",
  "api_version": "v1",
  "version": "4.2.0",
  "tasks_queued": 0
}
```

`client.get_task(task_id)`:

```json
{
  "task_id": "8d8c7a20-34cf-4f75-9d64-9b1e2d7e16dd",
  "status": "completed",
  "text": "collect recent error counts",
  "created_at": "2026-03-31T12:00:00Z",
  "result": "No recent critical errors"
}
```

## Error handling

```python
try:
    client.status()
except requests.HTTPError as exc:
    print(exc.agentos_code, exc)
```
