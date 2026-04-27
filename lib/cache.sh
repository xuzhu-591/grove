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

# Read ~/.groverc (global) + <worktree>/.groverc (project), merged and deduped
_grove_read_groverc() {
    local dir="$1"
    { _grove_parse_rc_file "$HOME/.groverc"; _grove_parse_rc_file "$dir/.groverc"; } | awk '!seen[$0]++'
}

# Symlink cache dirs from source to target worktree
grove_link_cache() {
    local source_dir="$1" target_dir="$2"

    local dirs=()
    while IFS= read -r d; do
        dirs+=("$d")
    done < <(_grove_read_groverc "$source_dir")

    if [[ ${#dirs[@]} -eq 0 ]]; then
        return 0
    fi

    local linked=0
    for d in "${dirs[@]}"; do
        local src="$source_dir/$d"
        local dst="$target_dir/$d"

        if [[ ! -e "$src" ]]; then
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

    local dirs=()
    while IFS= read -r d; do
        dirs+=("$d")
    done < <(_grove_read_groverc "$main_dir")

    if [[ ${#dirs[@]} -eq 0 ]]; then
        grove_warn "No .groverc found (checked ~/.groverc and project root)"
        return 0
    fi

    for d in "${dirs[@]}"; do
        local dst="$wt_dir/$d"
        local src="$main_dir/$d"
        if [[ -L "$dst" ]]; then
            local target
            target=$(readlink "$dst")
            echo -e "  ${GREEN}linked${RESET}  $d -> $(grove_short_path "$target")" >&2
        elif [[ -d "$dst" ]]; then
            echo -e "  ${YELLOW}local${RESET}   $d" >&2
        elif [[ -e "$src" ]]; then
            echo -e "  ${RED}missing${RESET} $d (available in main)" >&2
        else
            echo -e "  ${DIM}N/A${RESET}     $d (not in main either)" >&2
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
    done < <(_grove_read_groverc "$main_dir")

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
