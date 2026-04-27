#!/usr/bin/env zsh
# grove.zsh - shell integration
# Source this file in .zshrc to enable `grove` as a shell function
# that can change the working directory.
#
# Usage: source /path/to/grove/shell/grove.zsh

GROVE_ROOT="${GROVE_ROOT:-$(cd "$(dirname "${(%):-%x}")/.." && pwd)}"

grove() {
    local grove_exec="${GROVE_ROOT}/bin/grove"

    # Commands that might emit __GROVE_CD__ directive
    case "${1:-}" in
        switch|cd|add|new|remove|rm)
            local output rc
            output=$("$grove_exec" "$@")
            rc=$?

            if [[ $rc -ne 0 ]]; then
                # Error output already went to stderr; show any stdout too
                [[ -n "$output" ]] && echo "$output"
                return $rc
            fi

            # Check last line for cd directive
            local last_line="${output##*$'\n'}"
            if [[ "$last_line" == __GROVE_CD__:* ]]; then
                # Print everything except the cd directive
                local rest="${output%$'\n'$last_line}"
                [[ -n "$rest" && "$rest" != "$last_line" ]] && echo "$rest"
                # Execute cd
                local target_dir="${last_line#__GROVE_CD__:}"
                builtin cd "$target_dir"
            else
                # No cd directive, print as-is
                [[ -n "$output" ]] && echo "$output"
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
