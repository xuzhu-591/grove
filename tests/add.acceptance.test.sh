#!/usr/bin/env bash
# Acceptance tests for `grove add` — remote branch handling
#
# Scenarios:
#   1. (P0 - Bug fix) grove --plain add <branch> --remote succeeds when
#      the local branch already exists
#   2. (P0 - Regression) grove --plain add <branch> --remote creates a new
#      tracking branch when it does not exist locally
#   3. (P0 - Regression) grove --plain add <branch> (no --remote) still
#      works normally
#
# Exit codes: 0 = all pass, 1 = any fail.

set -uo pipefail

# ── Globals ──────────────────────────────────────────────────────────────

GROVE_BIN="$(cd "$(dirname "$0")/.." && pwd)/bin/grove"
TMPDIR_ROOT=""
PASS_COUNT=0
FAIL_COUNT=0

# ── Helpers ──────────────────────────────────────────────────────────────

cleanup() {
    if [[ -n "$TMPDIR_ROOT" && -d "$TMPDIR_ROOT" ]]; then
        rm -rf "$TMPDIR_ROOT"
    fi
    # Clean up any worktrees we created under GROVE_WORKTREE_BASE
    if [[ -n "${GROVE_WORKTREE_BASE:-}" && -d "$GROVE_WORKTREE_BASE" ]]; then
        rm -rf "$GROVE_WORKTREE_BASE"
    fi
}
trap cleanup EXIT

pass() {
    echo "PASS: $1"
    (( PASS_COUNT++ ))
}

fail() {
    echo "FAIL: $1"
    [[ -n "${2:-}" ]] && echo "      $2"
    (( FAIL_COUNT++ ))
}

# ── Fixture: build a repo with multiple remotes ─────────────────────────
#
# Layout:
#   bare_origin/   - bare repo acting as "origin"
#   bare_second/   - bare repo acting as "second" remote
#   work_repo/     - the working clone (this is where we run grove)
#
# Both remotes have a "main" branch.  "second" also has a branch called
# "feat/only-on-second" that does NOT exist locally.
#
setup_fixture() {
    TMPDIR_ROOT="$(mktemp -d)"
    export GROVE_WORKTREE_BASE="$TMPDIR_ROOT/grove_worktrees"
    mkdir -p "$GROVE_WORKTREE_BASE"

    local bare_origin="$TMPDIR_ROOT/bare_origin"
    local bare_second="$TMPDIR_ROOT/bare_second"
    local work_repo="$TMPDIR_ROOT/work_repo"

    # ── Create "origin" bare repo with an initial commit on main ──
    git init --bare "$bare_origin" -b main >/dev/null 2>&1

    # Use a temp clone to push initial content to origin
    local tmp_clone="$TMPDIR_ROOT/tmp_clone"
    git clone "$bare_origin" "$tmp_clone" >/dev/null 2>&1
    git -C "$tmp_clone" config user.email "test@test.com"
    git -C "$tmp_clone" config user.name "Test"
    echo "init" > "$tmp_clone/file.txt"
    git -C "$tmp_clone" add file.txt
    git -C "$tmp_clone" commit -m "initial" >/dev/null 2>&1
    git -C "$tmp_clone" push origin main >/dev/null 2>&1
    rm -rf "$tmp_clone"

    # ── Create "second" bare repo ──
    # Clone from origin so it shares history, then add a unique branch.
    git clone --bare "$bare_origin" "$bare_second" >/dev/null 2>&1

    # Push a unique branch to "second"
    local tmp_second="$TMPDIR_ROOT/tmp_second"
    git clone "$bare_second" "$tmp_second" >/dev/null 2>&1
    git -C "$tmp_second" config user.email "test@test.com"
    git -C "$tmp_second" config user.name "Test"
    git -C "$tmp_second" checkout -b "feat/only-on-second" >/dev/null 2>&1
    echo "second-only" > "$tmp_second/second.txt"
    git -C "$tmp_second" add second.txt
    git -C "$tmp_second" commit -m "second only branch" >/dev/null 2>&1
    git -C "$tmp_second" push origin "feat/only-on-second" >/dev/null 2>&1
    rm -rf "$tmp_second"

    # ── Create the working repo (the one grove operates on) ──
    git clone "$bare_origin" "$work_repo" >/dev/null 2>&1
    git -C "$work_repo" config user.email "test@test.com"
    git -C "$work_repo" config user.name "Test"
    git -C "$work_repo" remote add second "$bare_second" >/dev/null 2>&1
    git -C "$work_repo" fetch --all >/dev/null 2>&1

    WORK_REPO="$work_repo"
}

# ── Tests ────────────────────────────────────────────────────────────────

test_remote_add_when_local_branch_exists() {
    local desc="grove --plain add <remote_branch> --remote succeeds when local branch already exists"

    # The bug: when a local branch already exists and you run
    # `grove add <branch> --remote`, the code tried `git worktree add -b <branch>`,
    # which fails with "fatal: a branch named '<branch>' already exists".
    #
    # To trigger this, we need a branch that:
    #   (a) exists locally
    #   (b) exists on a remote
    #   (c) is NOT currently checked out in any worktree (otherwise git
    #       would complain about that instead)
    #
    # Strategy: create a local branch "feat/shared" and push it to "second"
    # remote, then switch the work repo to a different branch so "feat/shared"
    # is not checked out.

    git -C "$WORK_REPO" checkout -b "feat/shared" >/dev/null 2>&1
    echo "shared" > "$WORK_REPO/shared.txt"
    git -C "$WORK_REPO" add shared.txt
    git -C "$WORK_REPO" commit -m "shared branch commit" >/dev/null 2>&1

    # Push to "second" remote so it exists as second/feat/shared
    git -C "$WORK_REPO" push second "feat/shared" >/dev/null 2>&1

    # Switch back to main so feat/shared is not checked out
    git -C "$WORK_REPO" checkout main >/dev/null 2>&1

    # Now "feat/shared" exists both locally and on remote "second",
    # and it is NOT checked out. This is the exact scenario that
    # triggers the bug.
    local output
    output="$(cd "$WORK_REPO" && "$GROVE_BIN" --plain add "feat/shared" --remote 2>&1)" || {
        fail "$desc" "exit code $?, output: $output"
        return
    }

    # The command should have emitted a worktree path (grove_emit_cd in plain
    # mode prints the directory path to stdout).
    if [[ -z "$output" ]]; then
        fail "$desc" "no output (expected worktree path)"
        return
    fi

    # The emitted path should be a directory that exists.
    # grove_emit_cd in plain mode outputs the path on the last line of stdout.
    local wt_dir
    wt_dir="$(echo "$output" | tail -1)"
    if [[ ! -d "$wt_dir" ]]; then
        fail "$desc" "emitted path '$wt_dir' is not a directory"
        return
    fi

    # The worktree should be on the "feat/shared" branch.
    local branch
    branch="$(git -C "$wt_dir" rev-parse --abbrev-ref HEAD 2>/dev/null)"
    if [[ "$branch" != "feat/shared" ]]; then
        fail "$desc" "expected branch 'feat/shared', got '$branch'"
        return
    fi

    pass "$desc"

    # Cleanup this worktree so it does not interfere with other tests.
    git -C "$WORK_REPO" worktree remove "$wt_dir" --force 2>/dev/null || true
}

test_remote_add_new_tracking_branch() {
    local desc="grove --plain add <remote_branch> --remote creates new tracking branch when it does not exist locally"

    # "feat/only-on-second" exists only on the "second" remote.
    # grove add should create a local tracking branch and a worktree.

    local output
    output="$(cd "$WORK_REPO" && "$GROVE_BIN" --plain add "feat/only-on-second" --remote 2>&1)" || {
        fail "$desc" "exit code $?, output: $output"
        return
    }

    if [[ -z "$output" ]]; then
        fail "$desc" "no output (expected worktree path)"
        return
    fi

    local wt_dir
    wt_dir="$(echo "$output" | tail -1)"
    if [[ ! -d "$wt_dir" ]]; then
        fail "$desc" "emitted path '$wt_dir' is not a directory"
        return
    fi

    # The worktree should be on "feat/only-on-second".
    local branch
    branch="$(git -C "$wt_dir" rev-parse --abbrev-ref HEAD 2>/dev/null)"
    if [[ "$branch" != "feat/only-on-second" ]]; then
        fail "$desc" "expected branch 'feat/only-on-second', got '$branch'"
        return
    fi

    # The file from the second-only commit should be present.
    if [[ ! -f "$wt_dir/second.txt" ]]; then
        fail "$desc" "expected file 'second.txt' in worktree but it is missing"
        return
    fi

    pass "$desc"

    git -C "$WORK_REPO" worktree remove "$wt_dir" --force 2>/dev/null || true
}

test_local_add_without_remote_flag() {
    local desc="grove --plain add <branch> (without --remote) still works normally"

    # Create a new local branch in the work repo so we have something
    # to add without --remote.
    git -C "$WORK_REPO" branch "feat/local-test" >/dev/null 2>&1

    local output
    output="$(cd "$WORK_REPO" && "$GROVE_BIN" --plain add "feat/local-test" 2>&1)" || {
        fail "$desc" "exit code $?, output: $output"
        return
    }

    if [[ -z "$output" ]]; then
        fail "$desc" "no output (expected worktree path)"
        return
    fi

    local wt_dir
    wt_dir="$(echo "$output" | tail -1)"
    if [[ ! -d "$wt_dir" ]]; then
        fail "$desc" "emitted path '$wt_dir' is not a directory"
        return
    fi

    local branch
    branch="$(git -C "$wt_dir" rev-parse --abbrev-ref HEAD 2>/dev/null)"
    if [[ "$branch" != "feat/local-test" ]]; then
        fail "$desc" "expected branch 'feat/local-test', got '$branch'"
        return
    fi

    pass "$desc"

    git -C "$WORK_REPO" worktree remove "$wt_dir" --force 2>/dev/null || true
}

# ── Main ─────────────────────────────────────────────────────────────────

main() {
    echo "=== grove add acceptance tests ==="
    echo ""

    setup_fixture

    test_remote_add_when_local_branch_exists
    test_remote_add_new_tracking_branch
    test_local_add_without_remote_flag

    echo ""
    echo "--- Results: $PASS_COUNT passed, $FAIL_COUNT failed ---"

    if (( FAIL_COUNT > 0 )); then
        exit 1
    fi
    exit 0
}

main
