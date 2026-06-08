#!/usr/bin/env bash
# grove installer
set -euo pipefail

GROVE_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN_SRC="$GROVE_ROOT/target/release/grove"
BIN_DIR="$HOME/.local/bin"
ZSHRC="$HOME/.zshrc"
MARKER="# grove shell integration"

echo "grove installer"
echo "==============="
echo ""

# Build if release binary not found
if [[ ! -f "$BIN_SRC" ]]; then
    echo "Building grove..."
    cargo build --release --manifest-path "$GROVE_ROOT/Cargo.toml"
fi

# Symlink binary
mkdir -p "$BIN_DIR"
ln -sf "$BIN_SRC" "$BIN_DIR/grove"
echo "[ok] symlinked to $BIN_DIR/grove"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^${BIN_DIR}$"; then
    echo "[warn] $BIN_DIR is not in PATH"
    echo "       add to .zshrc:  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Add shell integration
if grep -q "$MARKER" "$ZSHRC" 2>/dev/null; then
    echo "[ok] shell integration already in .zshrc"
else
    cat >> "$ZSHRC" <<SHELLEOF

$MARKER
source "$GROVE_ROOT/shell/grove.zsh"
SHELLEOF
    echo "[ok] added shell integration to .zshrc"
fi

echo ""
echo "Done! Run 'source ~/.zshrc' or open a new terminal."
echo ""
echo "Quick start:"
echo "  grove list          # show worktrees with status"
echo "  grove add           # create a worktree (interactive)"
echo "  grove switch        # jump to a worktree (interactive)"
echo "  grove remove        # remove a worktree (interactive)"
echo ""
echo "For AI/script use, add --plain:"
echo "  grove --plain list"
echo "  grove --plain add <branch> --create"
