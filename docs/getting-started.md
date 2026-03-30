# Getting Started with AgentOS

Welcome to AgentOS — a desktop AI agent that executes tasks on your PC via natural language.

## Installation

### Prerequisites
- Windows 10/11 (macOS and Linux coming soon)
- 8 GB RAM minimum
- An LLM API key (OpenAI, Anthropic, or local model)

### Install from Release
1. Download the latest `.msi` installer from the [Releases](https://github.com/AresEkb/AgentOS/releases) page.
2. Run the installer and follow the prompts.
3. Launch **AgentOS** from the Start Menu.

### Build from Source
```bash
git clone https://github.com/AresEkb/AgentOS.git
cd AgentOS
npm install
cd src-tauri && cargo build --release
npm run tauri build
```

## API Key Setup

1. Open AgentOS and navigate to **Settings > LLM Provider**.
2. Select your provider (OpenAI, Anthropic, Ollama, or custom).
3. Paste your API key and click **Save**.
4. Alternatively, set the environment variable:
   ```bash
   export AGENTOS_API_KEY="sk-..."
   ```

## Sending Your First Task

### From the UI
Type a task in the chat box and press Enter:
```
check disk space on C: drive
```

### From the REST API
```bash
curl -X POST http://localhost:8080/v1/message \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{"text": "check disk space on C: drive"}'
```

### From the Python SDK
```python
from agentos_sdk import AgentOS

agent = AgentOS(host="http://localhost:8080", api_key="aos_yourkey")
result = agent.send_task("check disk space on C: drive")
print(result)
```

## Playbooks

Playbooks are reusable sequences of tasks saved as YAML files.

### Example: `daily-health.yml`
```yaml
name: Daily Health Check
steps:
  - task: check disk space
  - task: check CPU usage
  - task: list running services
  - task: check network connectivity
```

### Running a Playbook
```bash
curl -X POST http://localhost:8080/v1/playbook/run \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{"playbook": "daily-health"}'
```

## Chains

Chains let you pipe the output of one task into the next using `$prev`.

```yaml
name: Disk Cleanup Chain
steps:
  - task: find files larger than 100MB in C:\Temp
  - task: summarize these files — $prev
  - task: delete temp files older than 30 days
```

Chains execute sequentially. If any step fails, the chain stops and reports the error.

## Next Steps

- [Python SDK Reference](sdk/python-sdk.md)
- [JavaScript SDK Reference](sdk/javascript-sdk.md)
- [Automation with Triggers](guides/automation-with-triggers.md)
- [Mesh Networking](guides/mesh-networking.md)
- [Building Plugins](guides/building-plugins.md)
- [Smart Playbooks](guides/smart-playbooks.md)
- [API Playground](api-playground.html) — test endpoints in your browser
