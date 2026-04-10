#!/usr/bin/env bash
# grove switch - jump to worktree

grove_switch() {
    grove_ensure_git || return 1

    if [[ "$GROVE_PLAIN" == true ]]; then
        _grove_switch_plain "$@"
    else
        _grove_switch_fzf "$@"
    fi
}

# Plain mode: grove switch --plain <branch>
_grove_switch_plain() {
    local branch=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -*) grove_error "grove switch: unknown option '$1'"; return 1 ;;
            *)  branch="$1"; shift ;;
        esac
    done

    if [[ -z "$branch" ]]; then
        grove_error "grove switch: branch name required"
        grove_error "usage: grove switch --plain <branch>"
        return 1
    fi

    local dir
    dir=$(grove_find_by_branch "$branch") || {
        grove_error "grove switch: no worktree for branch '$branch'"
        return 1
    }

    grove_emit_cd "$dir"
}

# FZF mode: interactive selection with status preview
_grove_switch_fzf() {
    grove_parse_worktrees

    if [[ ${#_grove_wt_dirs[@]} -eq 0 ]]; then
        grove_error "grove switch: no worktrees found"
        return 1
    fi

    # Build display lines: branch -> dir
    local lines=()
    for i in "${!_grove_wt_dirs[@]}"; do
        local short_dir="${_grove_wt_dirs[$i]/#$HOME/~}"
        lines+=("${_grove_wt_branches[$i]}  ${short_dir}")
    done

    local selected
    selected=$(printf '%s\n' "${lines[@]}" | \
        fzf --height=40% --reverse --border \
            --prompt="Worktree > " \
            --preview='dir=$(echo {} | awk "{print \$NF}"); dir="${dir/#\~/$HOME}"; git -C "$dir" log --oneline -5 2>/dev/null' \
            --preview-window=right:40%) || return 0

    # Extract branch name (first field)
    local branch
    branch=$(echo "$selected" | awk '{print $1}')

    local dir
    dir=$(grove_find_by_branch "$branch") || {
        grove_error "grove switch: worktree not found"
        return 1
    }

    grove_info "-> $dir"
    grove_emit_cd "$dir"
}
