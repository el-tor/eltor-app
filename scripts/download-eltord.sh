#!/bin/bash

# Script to download eltord binary from GitHub releases
# Usage: ./download-eltord.sh [version] [platform]
# Example: ./download-eltord.sh v0.0.1 macos-arm64

set -e

REPO="el-tor/eltord"
VERSION="${1:-latest}"
PLATFORM="${2:-auto}"

# Detect platform if not specified
if [ "$PLATFORM" = "auto" ]; then
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        darwin)
            case "$ARCH" in
                arm64)
                    PLATFORM="macos-arm64"
                    ;;
                x86_64)
                    PLATFORM="macos-x86_64"
                    ;;
                *)
                    echo "❌ Unsupported macOS architecture: $ARCH"
                    exit 1
                    ;;
            esac
            ;;
        linux)
            case "$ARCH" in
                aarch64|arm64)
                    PLATFORM="linux-arm64"
                    ;;
                x86_64|amd64)
                    PLATFORM="linux-x86_64"
                    ;;
                *)
                    echo "❌ Unsupported Linux architecture: $ARCH"
                    exit 1
                    ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            PLATFORM="windows-x86_64"
            ;;
        *)
            echo "❌ Unsupported OS: $OS"
            exit 1
            ;;
    esac
fi

echo "🔍 Detected platform: $PLATFORM"

# Get latest release version if not specified
if [ "$VERSION" = "latest" ]; then
    echo "🔍 Fetching latest release version..."
    
    # Try with authentication if GITHUB_TOKEN is set
    if [ -n "$GITHUB_TOKEN" ]; then
        VERSION=$(curl -s -H "Authorization: token $GITHUB_TOKEN" \
            "https://api.github.com/repos/$REPO/releases/latest" | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        # Try without authentication first
        VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    fi
    
    # Fallback to known version if API fails
    if [ -z "$VERSION" ]; then
        echo "⚠️  Failed to fetch from GitHub API (rate limited?), using fallback version v0.0.2"
        VERSION="v0.0.2"
    fi
fi

echo "📦 Downloading eltord $VERSION for $PLATFORM..."

# Create download directory
DOWNLOAD_DIR="$(dirname "$0")/../backend/bin"
mkdir -p "$DOWNLOAD_DIR"

# Construct download URL
ZIP_NAME="eltord-${PLATFORM}.zip"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ZIP_NAME"

echo "🌐 URL: $DOWNLOAD_URL"

# Download the zip file
TEMP_ZIP="/tmp/eltord-${PLATFORM}.zip"
if ! curl -L -f -o "$TEMP_ZIP" "$DOWNLOAD_URL"; then
    echo "❌ Failed to download eltord binary"
    echo "   Make sure the release exists at: $DOWNLOAD_URL"
    exit 1
fi

echo "📂 Extracting binary..."

# Extract just the eltord binary
if [ "$PLATFORM" = "windows-x86_64" ]; then
    unzip -o -j "$TEMP_ZIP" "eltord.exe" -d "$DOWNLOAD_DIR"
    BINARY_NAME="eltord.exe"
else
    unzip -o -j "$TEMP_ZIP" "eltord" -d "$DOWNLOAD_DIR"
    BINARY_NAME="eltord"
    chmod +x "$DOWNLOAD_DIR/$BINARY_NAME"
fi

# Cleanup
rm "$TEMP_ZIP"

echo "✅ Successfully downloaded eltord to $DOWNLOAD_DIR/$BINARY_NAME"
echo "📏 Binary size: $(ls -lh "$DOWNLOAD_DIR/$BINARY_NAME" | awk '{print $5}')"

# Verify the binary
if [ "$PLATFORM" != "windows-x86_64" ]; then
    echo "🔍 Verifying binary..."
    if "$DOWNLOAD_DIR/$BINARY_NAME" --version 2>/dev/null || "$DOWNLOAD_DIR/$BINARY_NAME" --help 2>/dev/null; then
        echo "✅ Binary verification passed"
    else
        echo "⚠️  Binary verification skipped (might need dependencies)"
    fi
fi
