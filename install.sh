#!/bin/bash
set -e

OWNER="sonesuke"
REPO="arxiv-cli"
BINARY_NAME="arxiv-cli"

# Detect OS and Arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
    linux) 
        if [ "$ARCH" = "arm64" ]; then
            echo "Error: Linux ARM64 is not supported. Use x86_64 or macOS."
            exit 1
        fi
        PLATFORM="linux-${ARCH}" 
        ;;
    darwin) PLATFORM="macos-${ARCH}" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

echo "Detecting latest version..."
LATEST_TAG=$(curl -s "https://api.github.com/repos/$OWNER/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "Failed to fetch latest version."
    exit 1
fi

echo "Downloading $BINARY_NAME $LATEST_TAG for $PLATFORM..."
ASSET_NAME="${BINARY_NAME}-${PLATFORM}.tar.gz"
DOWNLOAD_URL="https://github.com/$OWNER/$REPO/releases/download/$LATEST_TAG/$ASSET_NAME"

TEMP_DIR=$(mktemp -d)
curl -L "$DOWNLOAD_URL" -o "$TEMP_DIR/$ASSET_NAME"
tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$TEMP_DIR"

if [ "$OS" = "linux" ]; then
    # User-local installation for Linux without sudo
    INSTALL_DIR="$HOME/.local/bin"
else
    # On macOS, try /usr/local/bin if writable (no sudo required), else fallback to ~/.local/bin
    if [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi
fi

echo "Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
mv "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

rm -rf "$TEMP_DIR"
echo "Successfully installed $BINARY_NAME $LATEST_TAG to $INSTALL_DIR"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "WARNING: $INSTALL_DIR is not in your PATH."
    echo "Please add the following line to your shell profile (e.g., ~/.bashrc, ~/.zshrc):"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\""
fi
