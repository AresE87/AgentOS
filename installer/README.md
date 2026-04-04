# AgentOS Installer

All-in-one setup for AgentOS dependencies on Windows.

## What it installs

1. **Docker Desktop** -- container runtime for sandboxed worker execution
2. **Ollama** -- local LLM inference server
3. **Worker image** (`agentos-worker:latest`) -- Ubuntu-based container with Ollama, Chromium, Python, Node.js
4. **AI models** -- `phi3:mini` and `llama3.2:1b` via Ollama

## Usage

Run from PowerShell (as Administrator recommended):

```powershell
.\setup_docker.ps1
```

The script is idempotent -- it skips components that are already installed.

## Worker Image

The `worker-image/Dockerfile` builds a container with:

- Ubuntu 22.04 base
- Chromium + Xvfb for headless browsing
- Python 3 + pip
- Node.js + npm
- Ollama for local LLM inference
- Git, jq, and common CLI tools

## Requirements

- Windows 10/11 (64-bit)
- Internet connection for downloads
- Admin privileges for Docker Desktop installation
