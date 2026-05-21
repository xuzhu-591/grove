#!/usr/bin/env zsh
# grove.zsh - shell integration
# Source this file in .zshrc to enable `grove` as a shell function
# that can change the working directory.
#
# Usage: source /path/to/grove/shell/grove.zsh

GROVE_ROOT="${GROVE_ROOT:-$(cd "$(dirname "${(%):-%x}")/.." && pwd)}"

grove() {
    local grove_exec="${GROVE_ROOT}/bin/grove"

    # Commands that might need cd or interactive input
    case "${1:-}" in
        switch|cd|add|new|remove|rm)
            local _grove_cd_file _grove_read_file rc
            _grove_cd_file=$(mktemp)
            _grove_read_file=$(mktemp)
            trap "rm -f '$_grove_cd_file' '$_grove_read_file'" INT TERM

            GROVE_CD_FILE="$_grove_cd_file" \
            GROVE_READ_FILE="$_grove_read_file" \
                "$grove_exec" "$@"
            rc=$?

            # Script requested interactive input (rc=201)
            if [[ $rc -eq 201 && -s "$_grove_read_file" ]]; then
                local _grove_prompt=$(<"$_grove_read_file")
                local _grove_input=""
                vared -p "$_grove_prompt" _grove_input
                if [[ -n "$_grove_input" ]]; then
                    GROVE_CD_FILE="$_grove_cd_file" \
                        "$grove_exec" --plain add "$_grove_input" --create
                    rc=$?
                else
                    rc=0
                fi
            fi

            local cd_target=""
            [[ -s "$_grove_cd_file" ]] && cd_target=$(<"$_grove_cd_file")
            rm -f "$_grove_cd_file" "$_grove_read_file"
            trap - INT TERM

            if [[ $rc -ne 0 ]]; then
                return $rc
            fi

            if [[ -n "$cd_target" ]]; then
                builtin cd "$cd_target"
            fi
            ;;
        *)
            # Commands that don't change directory - pass through
            "$grove_exec" "$@"
            ;;
    esac
}

# Tab completion
_grove() {
    local -a commands=(
        'list:List worktrees with rich status'
        'add:Create a new worktree'
        'switch:Switch to a worktree'
        'remove:Remove a worktree'
        'cache:Manage build cache symlinks'
        'help:Show help'
        'version:Show version'
    )
    local -a global_flags=('--plain' '--fzf' '--help' '--version')

    if (( CURRENT == 2 )); then
        _describe 'command' commands
        _values 'flags' $global_flags
    elif (( CURRENT == 3 )); then
        case "${words[2]}" in
            switch|cd|remove|rm)
                # Complete with branch names from worktree list
                local -a branches
                branches=($(git worktree list --porcelain 2>/dev/null | \
                    grep '^branch ' | sed 's|^branch refs/heads/||'))
                _values 'branch' $branches
                ;;
            add|new)
                local -a branches flags
                branches=($(git branch --format='%(refname:short)' 2>/dev/null))
                branches+=($(git branch -r --format='%(refname:short)' 2>/dev/null | grep -v '/HEAD$'))
                flags=('--create' '--remote' '--no-cache')
                _values 'branch' $branches
                _values 'flags' $flags
                ;;
            cache)
                local -a cache_flags=('--status' '--unlink')
                _values 'flags' $cache_flags
                ;;
        esac
    fi
}
compdef _grove grove

# Short aliases
alias wls='grove ls'
alias wnw='grove new'
alias wcd='grove cd'
alias wrm='grove rm'

# Tab completion for short aliases
_wnw() {
    local -a branches
    branches=($(git branch --format='%(refname:short)' 2>/dev/null))
    branches+=($(git branch -r --format='%(refname:short)' 2>/dev/null | grep -v '/HEAD$'))
    _values 'branch' $branches
}
compdef _wnw wnw

_wcd() {
    local -a branches
    branches=($(git worktree list --porcelain 2>/dev/null | \
        grep '^branch ' | sed 's|^branch refs/heads/||'))
    _values 'branch' $branches
}
compdef _wcd wcd

_wrm() {
    local -a branches
    branches=($(git worktree list --porcelain 2>/dev/null | \
        grep '^branch ' | sed 's|^branch refs/heads/||'))
    _values 'branch' $branches
}
compdef _wrm wrm
