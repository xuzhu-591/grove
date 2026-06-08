#!/usr/bin/env bash
# tests/e2e/smoke.sh — grove end-to-end smoke test
set -uo pipefail

TMPDIR=$(mktemp -d)
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT

export HOME="$TMPDIR/home"
mkdir -p "$HOME"
export GROVE_WORKTREE_BASE="$TMPDIR/grove_worktrees"
mkdir -p "$GROVE_WORKTREE_BASE"

GROVE_BIN="$(cd "$(dirname "$0")/../.." && pwd)/target/debug/grove"
if [[ ! -f "$GROVE_BIN" ]]; then
    GROVE_BIN="$(cd "$(dirname "$0")/../.." && pwd)/target/release/grove"
fi
echo "Using grove at: $GROVE_BIN"

PASS=0
FAIL=0

pass() { echo "PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "FAIL: $1"; FAIL=$((FAIL + 1)); [[ -n "${2:-}" ]] && echo "      $2"; }

BARE="$TMPDIR/bare_origin"
WORK="$TMPDIR/work_repo"

git init --bare "$BARE" -b main >/dev/null 2>&1

TMP_CLONE="$TMPDIR/tmp_clone"
git clone "$BARE" "$TMP_CLONE" >/dev/null 2>&1
git -C "$TMP_CLONE" config user.email "test@test.com"
git -C "$TMP_CLONE" config user.name "Test"
echo "hello" > "$TMP_CLONE/README.md"
git -C "$TMP_CLONE" add README.md
git -C "$TMP_CLONE" commit -m "init" >/dev/null 2>&1
git -C "$TMP_CLONE" push origin main >/dev/null 2>&1
rm -rf "$TMP_CLONE"

git clone "$BARE" "$WORK" >/dev/null 2>&1
git -C "$WORK" config user.email "test@test.com"
git -C "$WORK" config user.name "Test"
cd "$WORK"

# Test 1: grove list
if output=$("$GROVE_BIN" --plain list 2>/dev/null) && echo "$output" | grep -q "main"; then
    pass "grove --plain list shows main worktree"
else
    fail "grove --plain list" "${output:-no output}"
fi

# Test 2: grove add --create
# project_name is basename of origin URL = "bare_origin"
# worktree path = GROVE_WORKTREE_BASE/bare_origin/feat-e2e
EXPECTED_DIR="$GROVE_WORKTREE_BASE/bare_origin/feat-e2e"
if output=$("$GROVE_BIN" --plain add "feat/e2e" --create 2>/dev/null); then
    # Resolve symlinks for comparison (macOS /var → /private/var)
    resolved=$(cd "$EXPECTED_DIR" 2>/dev/null && pwd -P) || true
    expected_resolved=$(cd "$(dirname "$EXPECTED_DIR")" 2>/dev/null && pwd -P)/feat-e2e || true
    if [[ -d "$EXPECTED_DIR" ]]; then
        pass "grove add --create feat/e2e"
    else
        fail "grove add --create feat/e2e" "expected dir: $EXPECTED_DIR"
    fi
else
    fail "grove add --create feat/e2e" "command failed: ${output:-}"
fi

# Test 3: grove switch
output=$("$GROVE_BIN" --plain switch "feat/e2e" 2>/dev/null) || true
# Compare resolved paths (handles /var vs /private/var on macOS)
switch_target=$(echo "${output:-}" | head -1)
switch_resolved=$(cd "$switch_target" 2>/dev/null && pwd -P 2>/dev/null) || true
expected_resolved=$(cd "$EXPECTED_DIR" 2>/dev/null && pwd -P 2>/dev/null) || true
if [[ "${switch_resolved:-}" == "${expected_resolved:-}" ]]; then
    pass "grove switch feat/e2e"
else
    fail "grove switch feat/e2e" "expected: $expected_resolved, got: $switch_resolved"
fi

# Test 4: grove cache status
cat > "$WORK/grove.toml" <<'TOML'
[cache]
rules = ["node_modules"]
TOML
mkdir -p "$WORK/node_modules"
output=$("$GROVE_BIN" --plain cache status 2>&1) || true
# node_modules exists as real dir, not symlink → "local"
if echo "${output:-}" | grep -q "local"; then
    pass "grove cache status works"
else
    fail "grove cache status" "output: ${output:-}"
fi

# Test 5: grove remove
git checkout main >/dev/null 2>&1 || true
if output=$("$GROVE_BIN" --plain remove "feat/e2e" 2>&1); then
    if [[ ! -d "$EXPECTED_DIR" ]]; then
        pass "grove remove feat/e2e"
    else
        fail "grove remove feat/e2e" "dir still exists"
    fi
else
    fail "grove remove feat/e2e" "command failed: ${output:-}"
fi

echo ""
echo "--- Results: $PASS passed, $FAIL failed ---"
[[ $FAIL -eq 0 ]] || exit 1
