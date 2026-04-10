#!/usr/bin/env bash
# grove add - create worktree

grove_add() {
    grove_ensure_git || return 1

    if [[ "$GROVE_PLAIN" == true ]]; then
        _grove_add_plain "$@"
    else
        _grove_add_fzf "$@"
    fi
}

# Plain mode: grove add --plain <branch> [--create]
_grove_add_plain() {
    local branch="" create=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --create|-c) create=true; shift ;;
            -*) grove_error "grove add: unknown option '$1'"; return 1 ;;
            *)  branch="$1"; shift ;;
        esac
    done

    if [[ -z "$branch" ]]; then
        grove_error "grove add: branch name required"
        grove_error "usage: grove add --plain <branch> [--create]"
        return 1
    fi

    local wt_dir
    wt_dir=$(grove_worktree_path "$branch") || return 1

    if [[ "$create" == true ]]; then
        git worktree add -b "$branch" "$wt_dir" 2>&1 || {
            grove_error "grove add: failed to create worktree"
            return 1
        }
    else
        git worktree add "$wt_dir" "$branch" 2>&1 || {
            grove_error "grove add: failed to create worktree"
            return 1
        }
    fi

    grove_emit_cd "$wt_dir"
}

# FZF mode: interactive branch selection/creation
_grove_add_fzf() {
    local action
    action=$(printf '%s\n' "existing branch" "new branch" | \
        fzf --height=10% --reverse --border --prompt="Action > ") || return 0

    local branch
    if [[ "$action" == "existing branch" ]]; then
        branch=$(git branch --format='%(refname:short)' | \
            fzf --height=40% --reverse --border --prompt="Branch > ") || return 0
    else
        echo -n "New branch name: " >&2
        read -r branch </dev/tty
        [[ -z "$branch" ]] && return 0
    fi

    local wt_dir
    wt_dir=$(grove_worktree_path "$branch") || return 1

    if [[ "$action" == "new branch" ]]; then
        git worktree add -b "$branch" "$wt_dir" >&2 || {
            grove_error "grove add: failed"
            return 1
        }
    else
        git worktree add "$wt_dir" "$branch" >&2 || {
            grove_error "grove add: failed"
            return 1
        }
    fi

    grove_info "Created: $wt_dir"
    grove_emit_cd "$wt_dir"
}
