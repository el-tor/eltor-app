#!/bin/bash

# Tauri before-build hook script
# Downloads the appropriate eltord binary before building

set -e

echo "🚀 Tauri Pre-Build: Downloading eltord binary..."

# Get the project root (go up 3 levels from src-tauri/scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Run the download script
"$PROJECT_ROOT/scripts/download-eltord.sh"

echo "✅ Tauri Pre-Build: eltord binary ready"
