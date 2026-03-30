# Building Plugins

Extend AgentOS with custom plugins that add new capabilities, tools, and integrations.

## Plugin Structure

A plugin is a directory with a `manifest.json` and one or more handler files:

```
my-plugin/
  manifest.json
  handler.py        # or handler.js
  README.md         # optional
```

## Manifest File

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "A sample AgentOS plugin",
  "author": "Your Name",
  "entry": "handler.py",
  "permissions": ["filesystem.read", "network.http"],
  "triggers": ["on_task", "on_schedule"],
  "settings": {
    "api_url": {
      "type": "string",
      "description": "External API endpoint",
      "default": "https://api.example.com"
    }
  }
}
```

### Manifest Fields

| Field          | Required | Description                                    |
|----------------|----------|------------------------------------------------|
| `name`         | Yes      | Unique plugin identifier (kebab-case)          |
| `version`      | Yes      | Semver version string                          |
| `description`  | Yes      | Short description                              |
| `entry`        | Yes      | Path to handler file                           |
| `permissions`  | Yes      | Capabilities the plugin needs                  |
| `triggers`     | No       | Events that activate the plugin                |
| `settings`     | No       | User-configurable settings                     |

## Writing a Handler (Python)

```python
"""Example plugin handler."""

def on_task(context):
    """Called when a task matches this plugin."""
    task_text = context["task"]["text"]

    if "weather" in task_text.lower():
        city = extract_city(task_text)
        weather = fetch_weather(city)
        return {"result": weather, "status": "completed"}

    return None  # Not handled — pass to next plugin

def on_schedule(context):
    """Called on scheduled trigger."""
    return {"result": "Scheduled task executed"}

def extract_city(text):
    # Simple extraction logic
    words = text.split()
    idx = words.index("in") if "in" in words else -1
    return words[idx + 1] if idx >= 0 and idx + 1 < len(words) else "London"

def fetch_weather(city):
    import requests
    # Replace with your preferred weather API
    resp = requests.get(f"https://wttr.in/{city}?format=3")
    return resp.text
```

## Writing a Handler (JavaScript)

```javascript
export function onTask(context) {
  const text = context.task.text;

  if (text.toLowerCase().includes('weather')) {
    return { result: `Weather check for: ${text}`, status: 'completed' };
  }

  return null;
}

export function onSchedule(context) {
  return { result: 'Scheduled task executed' };
}
```

## Permissions

Plugins run in a sandboxed environment and must declare permissions:

| Permission          | Description                         |
|---------------------|-------------------------------------|
| `filesystem.read`   | Read files on disk                  |
| `filesystem.write`  | Write files on disk                 |
| `network.http`      | Make HTTP requests                  |
| `network.socket`    | Open TCP/UDP sockets                |
| `system.exec`       | Execute system commands             |
| `system.env`        | Access environment variables        |

## Installing a Plugin

### From the UI
1. Go to **Settings > Plugins**.
2. Click **Install Plugin**.
3. Select the plugin directory or drag-drop a `.zip` file.

### From the CLI
```bash
# Copy the plugin into the plugins directory
cp -r my-plugin/ ~/.agentos/plugins/my-plugin/

# Or install from a URL
curl -L https://example.com/my-plugin.zip -o /tmp/my-plugin.zip
unzip /tmp/my-plugin.zip -d ~/.agentos/plugins/
```

### From the Marketplace
```bash
curl -X POST http://localhost:8080/v1/marketplace/install \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{"plugin": "weather-checker", "version": "1.0.0"}'
```

## Testing Your Plugin

```bash
curl -X POST http://localhost:8080/v1/message \
  -H "Authorization: Bearer aos_yourkey" \
  -d '{"text": "what is the weather in Tokyo"}'
```

## Publishing to the Marketplace

1. Ensure your plugin has a `README.md` and passes validation:
   ```bash
   curl -X POST http://localhost:8080/v1/plugins/validate \
     -d '{"path": "./my-plugin"}'
   ```
2. Submit for review:
   ```bash
   curl -X POST http://localhost:8080/v1/marketplace/submit \
     -H "Authorization: Bearer aos_yourkey" \
     -d '{"path": "./my-plugin"}'
   ```
