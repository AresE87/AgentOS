#!/bin/bash
set -e

echo "=== AgentOS Release Build ==="
echo ""

# 1. Run tests
echo "--- Running tests ---"
cd src-tauri
cargo test
echo "Tests passed!"
cd ..

# 2. Build frontend
echo ""
echo "--- Building frontend ---"
cd frontend
npm ci
npm run build
cd ..

# 3. Build Tauri (release mode)
echo ""
echo "--- Building Tauri release ---"
cargo tauri build

echo ""
echo "=== Build complete ==="
echo ""
echo "Installers:"
ls -lh src-tauri/target/release/bundle/nsis/*.exe 2>/dev/null || echo "  (no NSIS installer found)"
ls -lh src-tauri/target/release/bundle/msi/*.msi 2>/dev/null || echo "  (no MSI installer found)"
