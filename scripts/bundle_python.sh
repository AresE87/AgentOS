#!/bin/bash
# AgentOS — Python Bundling Script
# Packages Python embeddable + dependencies for Tauri sidecar
set -e

PYTHON_VERSION="3.11.9"
PLATFORM=$(uname -s)
BUNDLE_DIR="src-tauri/resources"

echo "=== AgentOS Python Bundling ==="
echo "Python: ${PYTHON_VERSION}"
echo "Platform: ${PLATFORM}"

# Create bundle directory
mkdir -p "${BUNDLE_DIR}/python/Lib/site-packages"
mkdir -p "${BUNDLE_DIR}/config"
mkdir -p "${BUNDLE_DIR}/playbooks"

if [[ "$PLATFORM" == "MINGW"* ]] || [[ "$PLATFORM" == "MSYS"* ]] || [[ "$OS" == "Windows_NT" ]]; then
    echo "--- Downloading Python Embeddable (Windows) ---"
    EMBED_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-embed-amd64.zip"
    curl -sL -o /tmp/python-embed.zip "${EMBED_URL}"
    unzip -qo /tmp/python-embed.zip -d "${BUNDLE_DIR}/python/"

    # Enable site-packages in embeddable Python
    PTH_FILE=$(ls "${BUNDLE_DIR}/python/"python*._pth 2>/dev/null | head -1)
    if [ -n "$PTH_FILE" ]; then
        echo "import site" >> "$PTH_FILE"
    fi

    # Install pip
    curl -sL https://bootstrap.pypa.io/get-pip.py -o /tmp/get-pip.py
    "${BUNDLE_DIR}/python/python.exe" /tmp/get-pip.py --no-warn-script-location

    # Install dependencies
    echo "--- Installing dependencies ---"
    "${BUNDLE_DIR}/python/python.exe" -m pip install \
        --target "${BUNDLE_DIR}/python/Lib/site-packages" \
        --no-warn-script-location --quiet \
        litellm python-telegram-bot rich pyyaml httpx aiosqlite pydantic python-dotenv cryptography \
        mss pyautogui pynput Pillow numpy
else
    echo "--- Linux: Using system Python ---"
    # On Linux, we use a different strategy (venv or nuitka)
    python3 -m venv "${BUNDLE_DIR}/python"
    source "${BUNDLE_DIR}/python/bin/activate"
    pip install --quiet \
        litellm python-telegram-bot rich pyyaml httpx aiosqlite pydantic python-dotenv cryptography \
        mss pyautogui pynput Pillow numpy
fi

# Copy AgentOS source code
echo "--- Copying AgentOS code ---"
cp -r agentos/ "${BUNDLE_DIR}/python/Lib/site-packages/agentos/"

# Copy config files
echo "--- Copying config ---"
cp -r config/ "${BUNDLE_DIR}/config/"

# Copy example playbooks (only valid ones)
echo "--- Copying playbooks ---"
for pb in hello_world system_monitor code_reviewer; do
    if [ -d "examples/playbooks/${pb}" ]; then
        cp -r "examples/playbooks/${pb}" "${BUNDLE_DIR}/playbooks/"
    fi
done

echo "=== Bundle complete ==="
du -sh "${BUNDLE_DIR}"
