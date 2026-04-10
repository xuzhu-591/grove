#!/usr/bin/env bash
# grove list - rich worktree status display

grove_list() {
    grove_ensure_git || return 1
    grove_parse_worktrees

    if [[ "$GROVE_PLAIN" == true ]]; then
        _grove_list_plain
    else
        _grove_list_pretty
    fi
}

_grove_list_plain() {
    # TSV: branch \t dir \t commit \t staged=N \t modified=N \t untracked=N \t ahead=N \t behind=N
    for i in "${!_grove_wt_dirs[@]}"; do
        local dir="${_grove_wt_dirs[$i]}"
        local branch="${_grove_wt_branches[$i]}"
        local commit="${_grove_wt_commits[$i]}"
        local status
        read -r staged modified untracked ahead behind <<< "$(grove_worktree_status "$dir")"
        printf '%s\t%s\t%s\tstaged=%s\tmodified=%s\tuntracked=%s\tahead=%s\tbehind=%s\n' \
            "$branch" "$dir" "$commit" "$staged" "$modified" "$untracked" "$ahead" "$behind"
    done
}

_grove_list_pretty() {
    # Collect all data first for column alignment
    local -a branches=() dirs=() commits=() statuses=()
    local max_branch=6 max_dir=3  # header widths

    for i in "${!_grove_wt_dirs[@]}"; do
        local dir="${_grove_wt_dirs[$i]}"
        local branch="${_grove_wt_branches[$i]}"
        local commit="${_grove_wt_commits[$i]}"

        # Shorten home dir
        local short_dir="${dir/#$HOME/~}"

        read -r staged modified untracked ahead behind <<< "$(grove_worktree_status "$dir")"
        local status_str
        status_str=$(grove_format_status "$staged" "$modified" "$untracked" "$ahead" "$behind")

        branches+=("$branch")
        dirs+=("$short_dir")
        commits+=("$commit")
        statuses+=("$status_str")

        (( ${#branch} > max_branch )) && max_branch=${#branch}
        (( ${#short_dir} > max_dir )) && max_dir=${#short_dir}
    done

    # Cap max widths
    (( max_dir > 50 )) && max_dir=50

    # Header
    printf "${BOLD}  %-${max_branch}s  %-${max_dir}s  %-7s  %s${RESET}\n" \
        "BRANCH" "DIR" "COMMIT" "STATUS"

    # Rows
    for i in "${!branches[@]}"; do
        local is_main=""
        [[ $i -eq 0 ]] && is_main="${DIM}*${RESET}" || is_main=" "

        local display_dir="${dirs[$i]}"
        if (( ${#display_dir} > max_dir )); then
            display_dir="...${display_dir: -$((max_dir - 3))}"
        fi

        printf "${is_main} ${CYAN}%-${max_branch}s${RESET}  %-${max_dir}s  ${DIM}%s${RESET}  %s\n" \
            "${branches[$i]}" "$display_dir" "${commits[$i]}" "${statuses[$i]}"
    done
}
