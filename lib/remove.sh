#!/usr/bin/env bash
# grove remove - delete worktree with safety checks

grove_remove() {
    grove_ensure_git || return 1

    if [[ "$GROVE_PLAIN" == true ]]; then
        _grove_remove_plain "$@"
    else
        _grove_remove_fzf "$@"
    fi
}

# ---------- Safety checks ----------
# Returns 0 if safe, 1 if dirty. Messages to stderr.
_grove_safety_check() {
    local wt_dir="$1"

    # Uncommitted changes
    local dirty
    dirty=$(git -C "$wt_dir" status --porcelain 2>/dev/null)
    if [[ -n "$dirty" ]]; then
        grove_error "grove remove: worktree has uncommitted changes:"
        echo "$dirty" >&2
        return 1
    fi

    # Unpushed commits
    local upstream unpushed
    upstream=$(git -C "$wt_dir" rev-parse --abbrev-ref '@{u}' 2>/dev/null)
    if [[ -n "$upstream" ]]; then
        unpushed=$(git -C "$wt_dir" log '@{u}..HEAD' --oneline 2>/dev/null)
    else
        local main_dir
        main_dir=$(git worktree list | head -1 | awk '{print $1}')
        unpushed=$(git -C "$wt_dir" log "$(git -C "$main_dir" rev-parse HEAD)..HEAD" --oneline 2>/dev/null)
    fi
    if [[ -n "$unpushed" ]]; then
        grove_error "grove remove: worktree has unpushed commits:"
        echo "$unpushed" >&2
        return 1
    fi

    return 0
}

# Plain mode: grove remove --plain <branch> [--force]
_grove_remove_plain() {
    local branch="" force=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --force|-f) force=true; shift ;;
            -*) grove_error "grove remove: unknown option '$1'"; return 1 ;;
            *)  branch="$1"; shift ;;
        esac
    done

    if [[ -z "$branch" ]]; then
        grove_error "grove remove: branch name required"
        grove_error "usage: grove remove --plain <branch> [--force]"
        return 1
    fi

    local dir
    dir=$(grove_find_by_branch "$branch") || {
        grove_error "grove remove: no worktree for branch '$branch'"
        return 1
    }

    # Don't allow removing the main worktree
    local main_dir
    main_dir=$(git worktree list | head -1 | awk '{print $1}')
    if [[ "$dir" == "$main_dir" ]]; then
        grove_error "grove remove: cannot remove main worktree"
        return 1
    fi

    if [[ "$force" != true ]]; then
        _grove_safety_check "$dir" || return 1
    fi

    # If cwd is inside the worktree being removed, emit cd to main
    local need_cd=false
    if [[ "$(pwd)" == "$dir"* ]]; then
        need_cd=true
    fi

    if [[ "$force" == true ]]; then
        git worktree remove --force "$dir" 2>&1 || {
            grove_error "grove remove: failed"
            return 1
        }
    else
        git worktree remove "$dir" 2>&1 || {
            grove_error "grove remove: failed"
            return 1
        }
    fi

    echo "removed $dir" >&2
    [[ "$need_cd" == true ]] && grove_emit_cd "$main_dir"
    return 0
}

# FZF mode: interactive selection with safety checks
_grove_remove_fzf() {
    grove_parse_worktrees

    # Exclude main worktree (index 0)
    if [[ ${#_grove_wt_dirs[@]} -le 1 ]]; then
        grove_error "grove remove: no removable worktrees (main worktree cannot be removed)"
        return 1
    fi

    # Build display lines (skip index 0 = main)
    local lines=()
    for i in "${!_grove_wt_dirs[@]}"; do
        [[ $i -eq 0 ]] && continue
        local short_dir
        short_dir=$(grove_short_path "${_grove_wt_dirs[$i]}")
        lines+=("${_grove_wt_branches[$i]}  ${short_dir}")
    done

    local selected
    selected=$(printf '%s\n' "${lines[@]}" | \
        fzf --height=40% --reverse --border \
            --prompt="Remove worktree > " \
            --preview='dir=$(echo {} | awk "{print \$NF}"); dir="${dir/#\~/$HOME}"; echo "=== Status ==="; git -C "$dir" status -sb 2>/dev/null; echo; echo "=== Unpushed ==="; git -C "$dir" log --oneline "@{u}..HEAD" 2>/dev/null || echo "(no upstream)"' \
            --preview-window=right:50%) || return 0

    local branch
    branch=$(echo "$selected" | awk '{print $1}')

    local dir
    dir=$(grove_find_by_branch "$branch") || {
        grove_error "grove remove: worktree not found"
        return 1
    }

    _grove_safety_check "$dir" || return 1

    # cd away if needed
    local main_dir need_cd=false
    main_dir=$(git worktree list | head -1 | awk '{print $1}')
    if [[ "$(pwd)" == "$dir"* ]]; then
        need_cd=true
    fi

    git worktree remove "$dir" >&2 || {
        grove_error "grove remove: failed"
        return 1
    }

    grove_info "Removed: $dir"
    [[ "$need_cd" == true ]] && grove_emit_cd "$main_dir"
    return 0
}
