#!/bin/bash
# AgentOS — Full Build Script
# Builds frontend, bundles Python, and creates Tauri app
set -e

echo "======================================="
echo "     AgentOS Build Pipeline"
echo "======================================="
echo ""

# 1. Frontend build
echo "=== Step 1/3: Building Frontend ==="
cd frontend
npm ci --silent
npm run build
cd ..
echo "Frontend build complete."
echo ""

# 2. Python bundling
echo "=== Step 2/3: Bundling Python ==="
bash scripts/bundle_python.sh
echo "Python bundle complete."
echo ""

# 3. Tauri build
echo "=== Step 3/3: Building Tauri App ==="
cd src-tauri
cargo tauri build
cd ..

echo ""
echo "======================================="
echo "     Build Complete!"
echo "======================================="

# Show output
if [ -d "src-tauri/target/release/bundle/msi" ]; then
    echo "MSI installer:"
    ls -lh src-tauri/target/release/bundle/msi/*.msi 2>/dev/null
elif [ -d "src-tauri/target/release/bundle/deb" ]; then
    echo "DEB package:"
    ls -lh src-tauri/target/release/bundle/deb/*.deb 2>/dev/null
fi
