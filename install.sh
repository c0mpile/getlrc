#!/usr/bin/env bash
set -e

# getlrc Installation Script
# Installs the binary to ~/.local/bin and ensures it's in PATH

BINARY_NAME="getlrc"
INSTALL_DIR="$HOME/.local/bin"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== getlrc Installation ==="
echo ""

# Build release binary
echo "Building release binary..."
cd "$PROJECT_ROOT"
cargo build --release

# Create install directory if it doesn't exist
echo "Creating installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

# Copy binary
echo "Installing $BINARY_NAME to $INSTALL_DIR"
cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "✓ Installation complete!"
echo ""

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "⚠ WARNING: $INSTALL_DIR is not in your PATH"
    echo ""
    echo "Add the following line to your shell configuration file:"
    echo "  (~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish)"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then reload your shell configuration:"
    echo "  source ~/.bashrc  # or ~/.zshrc"
else
    echo "✓ $INSTALL_DIR is in your PATH"
fi

echo ""
echo "You can now run: $BINARY_NAME <music_directory>"
echo ""
echo "Data will be stored in: ~/.local/share/getlrc/"
