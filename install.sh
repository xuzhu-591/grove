#!/usr/bin/env bash
# grove installer
set -euo pipefail

GROVE_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$HOME/.local/bin"
ZSHRC="$HOME/.zshrc"

echo "grove installer"
echo "==============="
echo ""

# 1. Make executable
chmod +x "$GROVE_ROOT/bin/grove"
echo "[ok] bin/grove made executable"

# 2. Symlink to PATH
mkdir -p "$BIN_DIR"
ln -sf "$GROVE_ROOT/bin/grove" "$BIN_DIR/grove"
echo "[ok] symlinked to $BIN_DIR/grove"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^${BIN_DIR}$"; then
    echo "[warn] $BIN_DIR is not in PATH"
    echo "       add this to .zshrc:  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 3. Add shell integration to .zshrc
MARKER="# grove shell integration"
if grep -q "$MARKER" "$ZSHRC" 2>/dev/null; then
    echo "[ok] shell integration already in .zshrc"
else
    cat >> "$ZSHRC" <<EOF

$MARKER
export GROVE_ROOT="$GROVE_ROOT"
source "$GROVE_ROOT/shell/grove.zsh"
EOF
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
