#!/usr/bin/env bash
# grove cache - share build caches between worktrees via symlink

grove_cache() {
    grove_ensure_git || return 1

    if [[ "$GROVE_PLAIN" == true ]]; then
        _grove_cache_plain "$@"
    else
        _grove_cache_fzf "$@"
    fi
}

# ---------- Core logic ----------

# Parse a single .groverc file, output one dir name per line
_grove_parse_rc_file() {
    local rc_file="$1"
    [[ -f "$rc_file" ]] || return 0
    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%%#*}"
        line="${line#"${line%%[![:space:]]*}"}"
        line="${line%"${line##*[![:space:]]}"}"
        [[ -z "$line" ]] && continue
        echo "$line"
    done < "$rc_file"
}

# Read ~/.groverc (global) + <worktree>/.groverc (project), preserving rule order
_grove_read_groverc() {
    local dir="$1"
    _grove_parse_rc_file "$HOME/.groverc"
    _grove_parse_rc_file "$dir/.groverc"
}

_grove_is_negated_rule() {
    [[ "$1" == '!'* ]]
}

_grove_rule_pattern() {
    local rule="$1"
    if _grove_is_negated_rule "$rule"; then
        printf '%s\n' "${rule:1}"
    else
        printf '%s\n' "$rule"
    fi
}

_grove_is_safe_cache_rule() {
    local pattern="$1"
    local normalized="${pattern#/}"
    [[ -n "$normalized" ]] || return 1
    [[ "$normalized" != *"../"* ]] || return 1
    [[ "$normalized" != ".." ]] || return 1
    return 0
}

_grove_list_cache_candidates() {
    local source_dir="$1"
    find "$source_dir" -mindepth 1 -type d \( -path "$source_dir/.git" -o -path "$source_dir/.git/*" \) -prune -o -type d -print | while IFS= read -r dir; do
        dir="${dir#"$source_dir"/}"
        [[ -n "$dir" ]] && printf '%s\n' "$dir"
    done
}

_grove_rule_matches_path() {
    local pattern="$1" rel_path="$2"
    local anchored=false

    [[ "$pattern" == */ ]] && pattern="${pattern%/}"
    if [[ "$pattern" == /* ]]; then
        anchored=true
        pattern="${pattern#/}"
    fi
    [[ -n "$pattern" ]] || return 1

    if [[ "$pattern" == */'**' ]]; then
        local prefix="${pattern%/**}"
        [[ -n "$prefix" ]] || return 1
        if [[ "$anchored" == true ]]; then
            [[ "$rel_path" == "$prefix" || "$rel_path" == "$prefix"/* ]]
        else
            [[ "$rel_path" == "$prefix" || "$rel_path" == "$prefix"/* || "$rel_path" == */"$prefix" || "$rel_path" == */"$prefix"/* ]]
        fi
        return
    fi

    if [[ "$pattern" == '**/'* ]]; then
        local suffix="${pattern#**/}"
        [[ -n "$suffix" ]] || return 1
        [[ "$rel_path" == "$suffix" || "$rel_path" == */"$suffix" ]]
        return
    fi

    if [[ "$pattern" == *'/**/'* ]]; then
        [[ "$rel_path" == $pattern ]]
        return
    fi

    if [[ "$anchored" == true ]]; then
        [[ "$rel_path" == $pattern ]]
        return
    fi

    if [[ "$pattern" == *'/'* ]]; then
        [[ "$rel_path" == $pattern || "$rel_path" == */$pattern ]]
        return
    fi

    [[ "$rel_path" == "$pattern" || "$rel_path" == */"$pattern" ]]
}

_grove_resolve_cache_paths() {
    local source_dir="$1"
    local rules_file
    rules_file=$(mktemp)

    while IFS= read -r rule; do
        local pattern
        pattern=$(_grove_rule_pattern "$rule")
        _grove_is_safe_cache_rule "$pattern" || continue
        printf '%s\n' "$rule" >> "$rules_file"
    done < <(_grove_read_groverc "$source_dir")

    [[ -s "$rules_file" ]] || {
        rm -f "$rules_file"
        return 0
    }

    while IFS= read -r rel_path; do
        local selected=0
        while IFS= read -r rule; do
            local pattern
            pattern=$(_grove_rule_pattern "$rule")
            if _grove_rule_matches_path "$pattern" "$rel_path"; then
                if _grove_is_negated_rule "$rule"; then
                    selected=0
                else
                    selected=1
                fi
            fi
        done < "$rules_file"

        if (( selected == 1 )); then
            printf '%s\n' "$rel_path"
        fi
    done < <(_grove_list_cache_candidates "$source_dir")

    rm -f "$rules_file"
}

# Symlink cache dirs from source to target worktree
grove_link_cache() {
    local source_dir="$1" target_dir="$2"

    local dirs=()
    while IFS= read -r d; do
        dirs+=("$d")
    done < <(_grove_resolve_cache_paths "$source_dir")

    if [[ ${#dirs[@]} -eq 0 ]]; then
        return 0
    fi

    local linked=0
    for d in "${dirs[@]}"; do
        local src="$source_dir/$d"
        local dst="$target_dir/$d"

        if [[ ! -d "$src" ]]; then
            continue
        fi

        if [[ -e "$dst" || -L "$dst" ]]; then
            continue
        fi

        local parent
        parent=$(dirname "$dst")
        [[ -d "$parent" ]] || mkdir -p "$parent"

        ln -s "$src" "$dst" && (( linked++ ))
    done

    if [[ $linked -gt 0 ]]; then
        grove_info "Linked $linked cache dir(s) from $(grove_short_path "$source_dir")"
    fi
}

# ---------- Status display ----------

_grove_cache_status() {
    local wt_dir="$1"
    local main_dir
    main_dir=$(grove_main_worktree_dir)
    [[ -n "$main_dir" ]] || return 1

    local rules=()
    while IFS= read -r rule; do
        local pattern
        pattern=$(_grove_rule_pattern "$rule")
        _grove_is_safe_cache_rule "$pattern" || continue
        rules+=("$rule")
    done < <(_grove_read_groverc "$main_dir")

    if [[ ${#rules[@]} -eq 0 ]]; then
        grove_warn "No .groverc found (checked ~/.groverc and project root)"
        return 0
    fi

    local resolved=()
    while IFS= read -r path; do
        resolved+=("$path")
    done < <(_grove_resolve_cache_paths "$main_dir")
    local resolved_count=${#resolved[@]}

    local candidates=()
    while IFS= read -r c; do
        candidates+=("$c")
    done < <(_grove_list_cache_candidates "$main_dir")
    local candidates_count=${#candidates[@]}

    local rule
    for rule in "${rules[@]}"; do
        local pattern
        pattern=$(_grove_rule_pattern "$rule")

        if _grove_is_negated_rule "$rule"; then
            local excluded=0
            local c
            if (( candidates_count > 0 )); then for c in "${candidates[@]}"; do
                if _grove_rule_matches_path "$pattern" "$c"; then
                    (( excluded++ ))
                fi
            done; fi
            if (( excluded > 0 )); then
                echo -e "  ${YELLOW}exclude${RESET} $rule (${excluded} matched dir(s))" >&2
            else
                echo -e "  ${DIM}N/A${RESET}     $rule (no match)" >&2
            fi
            continue
        fi

        if [[ "$pattern" == *'*'* || "$pattern" == *'?'* ]]; then
            local matched=0
            local path
            if (( resolved_count > 0 )); then for path in "${resolved[@]}"; do
                if _grove_rule_matches_path "$pattern" "$path"; then
                    (( matched++ ))
                fi
            done; fi
            if (( matched > 0 )); then
                echo -e "  ${GREEN}matched${RESET} $rule (${matched} dir(s))" >&2
            else
                echo -e "  ${DIM}N/A${RESET}     $rule (no match)" >&2
            fi
            continue
        fi

        local dst="$wt_dir/$pattern"
        local src="$main_dir/$pattern"
        if [[ -L "$dst" ]]; then
            local target
            target=$(readlink "$dst")
            echo -e "  ${GREEN}linked${RESET}  $pattern -> $(grove_short_path "$target")" >&2
        elif [[ -d "$dst" ]]; then
            echo -e "  ${YELLOW}local${RESET}   $pattern" >&2
        elif [[ -d "$src" ]]; then
            echo -e "  ${RED}missing${RESET} $pattern (available in main)" >&2
        else
            echo -e "  ${DIM}N/A${RESET}     $pattern (not in main either)" >&2
        fi
    done
}

# ---------- Unlink ----------

_grove_cache_unlink() {
    local wt_dir="$1"
    local main_dir
    main_dir=$(grove_main_worktree_dir)

    local dirs=()
    while IFS= read -r d; do
        dirs+=("$d")
    done < <(_grove_resolve_cache_paths "$main_dir")

    local removed=0
    for d in "${dirs[@]}"; do
        local dst="$wt_dir/$d"
        if [[ -L "$dst" ]]; then
            rm "$dst" && (( removed++ ))
        fi
    done

    if [[ $removed -gt 0 ]]; then
        grove_info "Unlinked $removed cache dir(s)"
    else
        grove_info "No cache symlinks to remove"
    fi
}

# ---------- Plain mode ----------

_grove_cache_plain() {
    local action="link"
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --status|-s) action="status"; shift ;;
            --unlink|-u) action="unlink"; shift ;;
            -*) grove_error "grove cache: unknown option '$1'"; return 1 ;;
            *)  grove_error "grove cache: unexpected argument '$1'"; return 1 ;;
        esac
    done

    local wt_dir
    wt_dir=$(pwd)
    local main_dir
    main_dir=$(grove_main_worktree_dir)

    case "$action" in
        link)
            grove_link_cache "$main_dir" "$wt_dir"
            ;;
        status)
            _grove_cache_status "$wt_dir"
            ;;
        unlink)
            _grove_cache_unlink "$wt_dir"
            ;;
    esac
}

# ---------- FZF mode ----------

_grove_cache_fzf() {
    local action
    if [[ $# -gt 0 ]]; then
        case "$1" in
            --status|-s) action="status" ;;
            --unlink|-u) action="unlink" ;;
            -*) grove_error "grove cache: unknown option '$1'"; return 1 ;;
            *)  grove_error "grove cache: unexpected argument '$1'"; return 1 ;;
        esac
    else
        action=$(printf '%s\n' "link" "status" "unlink" | \
            fzf --height=10% --reverse --border --prompt="Cache action > ") || return 0
    fi

    local wt_dir
    wt_dir=$(pwd)
    local main_dir
    main_dir=$(grove_main_worktree_dir)

    case "$action" in
        link)
            grove_link_cache "$main_dir" "$wt_dir"
            ;;
        status)
            _grove_cache_status "$wt_dir"
            ;;
        unlink)
            _grove_cache_unlink "$wt_dir"
            ;;
    esac
}
