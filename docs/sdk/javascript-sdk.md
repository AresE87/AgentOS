# AgentOS JavaScript / Node.js SDK

Lightweight client for the AgentOS REST API, compatible with Node.js and modern browsers.

## Installation

```bash
npm install agentos-sdk
```

## Quick Start

```javascript
import { AgentOS } from 'agentos-sdk';

const agent = new AgentOS({
  host: 'http://localhost:8080',
  apiKey: 'aos_yourkey',
});

const result = await agent.sendTask('check disk space');
console.log(result);
```

## Configuration

```javascript
const agent = new AgentOS({
  host: 'http://localhost:8080',  // AgentOS server address
  apiKey: 'aos_yourkey',          // API key from Settings > API Keys
});
```

| Parameter | Type     | Default                  | Description              |
|-----------|----------|--------------------------|--------------------------|
| `host`    | `string` | `http://localhost:8080`  | AgentOS server URL       |
| `apiKey`  | `string` | `""`                     | Bearer token for auth    |

## Methods

### `agent.health(): Promise<object>`
Check if AgentOS is running.

```javascript
const status = await agent.health();
// { status: 'ok', version: '0.47.0' }
```

### `agent.status(): Promise<object>`
Get current agent status.

```javascript
const info = await agent.status();
// { state: 'idle', tasks_completed: 42, uptime: 3600 }
```

### `agent.sendTask(text: string): Promise<object>`
Send a natural-language task.

```javascript
const result = await agent.sendTask('list running processes');
// { task_id: 't_abc123', status: 'completed', result: '...' }
```

### `agent.getTask(taskId: string): Promise<object>`
Check the result of a previously submitted task.

```javascript
const task = await agent.getTask('t_abc123');
// { task_id: 't_abc123', status: 'completed', result: '...' }
```

## Error Handling

```javascript
try {
  const result = await agent.sendTask('check disk space');
} catch (error) {
  if (error.code === 'ECONNREFUSED') {
    console.error('Cannot connect to AgentOS — is it running?');
  } else {
    console.error('API error:', error.message);
  }
}
```

## CommonJS Usage

```javascript
const { AgentOS } = require('agentos-sdk');

const agent = new AgentOS({ apiKey: 'aos_yourkey' });
agent.sendTask('check disk space').then(console.log);
```

## Browser Usage

```html
<script type="module">
  import { AgentOS } from './agentos-sdk.mjs';

  const agent = new AgentOS({
    host: 'http://localhost:8080',
    apiKey: 'aos_yourkey',
  });

  const result = await agent.sendTask('check disk space');
  document.getElementById('output').textContent = JSON.stringify(result, null, 2);
</script>
```

## Reference Implementation

```javascript
class AgentOS {
  constructor({ host = 'http://localhost:8080', apiKey = '' } = {}) {
    this.host = host.replace(/\/+$/, '');
    this.apiKey = apiKey;
  }

  async _fetch(method, path, body) {
    const headers = { 'Content-Type': 'application/json' };
    if (this.apiKey) headers['Authorization'] = `Bearer ${this.apiKey}`;
    const opts = { method, headers };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(`${this.host}${path}`, opts);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    return res.json();
  }

  health()            { return this._fetch('GET', '/health'); }
  status()            { return this._fetch('GET', '/v1/status'); }
  sendTask(text)      { return this._fetch('POST', '/v1/message', { text }); }
  getTask(taskId)     { return this._fetch('GET', `/v1/task/${taskId}`); }
}

export { AgentOS };
```
