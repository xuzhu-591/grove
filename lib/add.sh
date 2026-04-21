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

# Plain mode: grove add --plain <branch> [--create] [--remote]
_grove_add_plain() {
    local branch="" create=false remote=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --create|-c) create=true; shift ;;
            --remote|-r) remote=true; shift ;;
            -*) grove_error "grove add: unknown option '$1'"; return 1 ;;
            *)  branch="$1"; shift ;;
        esac
    done

    if [[ -z "$branch" ]]; then
        grove_error "grove add: branch name required"
        grove_error "usage: grove add --plain <branch> [--create] [--remote]"
        return 1
    fi

    if [[ "$remote" == true ]]; then
        _grove_add_from_remote "$branch"
        return $?
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
    action=$(printf '%s\n' "existing branch" "new branch" "remote branch" | \
        fzf --height=10% --reverse --border --prompt="Action > ") || return 0

    local branch
    if [[ "$action" == "existing branch" ]]; then
        branch=$(git branch --format='%(refname:short)' | \
            fzf --height=40% --reverse --border --prompt="Branch > ") || return 0
    elif [[ "$action" == "remote branch" ]]; then
        grove_info "Fetching remote branches..."
        git fetch --all --prune >&2 || {
            grove_error "grove add: fetch failed"
            return 1
        }
        branch=$(git branch -r --format='%(refname:short)' | \
            grep -v '/HEAD$' | \
            fzf --height=40% --reverse --border --prompt="Remote branch > ") || return 0
        local local_branch="${branch#*/}"
        local wt_dir
        wt_dir=$(grove_worktree_path "$local_branch") || return 1
        git worktree add --track -b "$local_branch" "$wt_dir" "$branch" >&2 || {
            grove_error "grove add: failed"
            return 1
        }
        grove_info "Created: $wt_dir (tracking $branch)"
        grove_emit_cd "$wt_dir"
        return
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

# Shared: create worktree from a remote branch
_grove_add_from_remote() {
    local remote_branch="$1"
    grove_info "Fetching remote branches..."
    git fetch --all --prune >&2 || {
        grove_error "grove add: fetch failed"
        return 1
    }
    local local_branch="${remote_branch#*/}"
    local wt_dir
    wt_dir=$(grove_worktree_path "$local_branch") || return 1
    git worktree add --track -b "$local_branch" "$wt_dir" "$remote_branch" 2>&1 || {
        grove_error "grove add: failed to create worktree from remote branch"
        return 1
    }
    grove_info "Created: $wt_dir (tracking $remote_branch)"
    grove_emit_cd "$wt_dir"
}
