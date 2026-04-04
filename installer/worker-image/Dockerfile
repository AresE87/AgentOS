FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Base tools
RUN apt-get update && apt-get install -y \
    curl wget ca-certificates gnupg \
    chromium-browser xvfb \
    python3 python3-pip \
    nodejs npm \
    git jq unzip \
    && rm -rf /var/lib/apt/lists/*

# Ollama for local LLM inference
RUN curl -fsSL https://ollama.com/install.sh | bash

WORKDIR /workspace
EXPOSE 11434

# Start Ollama server + keep container alive
CMD ["bash", "-c", "ollama serve & sleep 2 && echo 'AgentOS Worker Ready' && exec tail -f /dev/null"]
