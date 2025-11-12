#!/bin/bash

# Script to download and build Arti from source
# Usage: ./download-arti.sh [platform]
# Example: ./download-arti.sh macos-arm64

set -e

ARTI_REPO="https://gitlab.torproject.org/tpo/core/arti.git"
PLATFORM="${1:-auto}"

# Check dependencies
check_dependencies() {
    echo "🔍 Checking dependencies..."
    
    if ! command -v git &> /dev/null; then
        echo "❌ git is required but not installed"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        echo "❌ Rust/Cargo is required but not installed"
        echo "   Install from: https://rustup.rs/"
        exit 1
    fi
    
    echo "✅ All dependencies found"
}

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

# Check dependencies before proceeding
check_dependencies

# Create download directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DOWNLOAD_DIR="$SCRIPT_DIR/../backend/bin"
mkdir -p "$DOWNLOAD_DIR"

echo "📁 Target directory: $DOWNLOAD_DIR"

# Download and build Arti
download_and_build_arti() {
    echo "🚀 Building Arti from source..."
    
    TEMP_DIR="/tmp/arti-build-$$"
    mkdir -p "$TEMP_DIR"
    
    echo "📥 Cloning Arti repository..."
    git clone "$ARTI_REPO" "$TEMP_DIR/arti"
    
    echo "🔨 Building Arti (this may take several minutes)..."
    cd "$TEMP_DIR/arti"
    
    # Build Arti
    cargo build -p arti --locked --release
    
    # Copy the binary to our target directory
    if [ "$PLATFORM" = "windows-x86_64" ]; then
        ARTI_BINARY="target/release/arti.exe"
        TARGET_BINARY="arti.exe"
    else
        ARTI_BINARY="target/release/arti"
        TARGET_BINARY="arti"
    fi
    
    if [ -f "$ARTI_BINARY" ]; then
        echo "🔍 Found Arti binary at: $ARTI_BINARY"
        echo "📁 Copying to: $DOWNLOAD_DIR/$TARGET_BINARY"
        cp "$ARTI_BINARY" "$DOWNLOAD_DIR/$TARGET_BINARY"
        if [ "$PLATFORM" != "windows-x86_64" ]; then
            chmod +x "$DOWNLOAD_DIR/$TARGET_BINARY"
        fi
        echo "✅ Arti binary copied to $DOWNLOAD_DIR/$TARGET_BINARY"
        echo "📏 Arti binary size: $(ls -lh "$DOWNLOAD_DIR/$TARGET_BINARY" | awk '{print $5}')"
    else
        echo "❌ Failed to build Arti binary at: $ARTI_BINARY"
        echo "🔍 Available files in target/release:"
        ls -la target/release/ | head -10
        cd - > /dev/null
        rm -rf "$TEMP_DIR"
        exit 1
    fi
    
    # Cleanup
    cd - > /dev/null
    rm -rf "$TEMP_DIR"
    
    echo "🎉 Arti successfully built and installed!"
    echo "📁 Binary location: $DOWNLOAD_DIR/$TARGET_BINARY"
}

# Main execution
echo "🎯 Starting Arti build process..."
download_and_build_arti