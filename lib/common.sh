#!/usr/bin/env bash
# grove - common utilities

# ---------- Colors (disabled in plain mode) ----------
if [[ "$GROVE_PLAIN" == true ]]; then
    RED="" GREEN="" YELLOW="" BLUE="" CYAN="" MAGENTA="" BOLD="" DIM="" RESET=""
else
    RED='\033[0;31m'    GREEN='\033[0;32m'   YELLOW='\033[0;33m'
    BLUE='\033[0;34m'   MAGENTA='\033[0;35m' CYAN='\033[0;36m'
    BOLD='\033[1m'      DIM='\033[2m'        RESET='\033[0m'
fi

# ---------- Logging (always to stderr) ----------
grove_info()  { echo -e "${GREEN}${1}${RESET}" >&2; }
grove_warn()  { echo -e "${YELLOW}${1}${RESET}" >&2; }

# ---------- Path helpers ----------
# Replace $HOME prefix with ~ (works in both bash and zsh)
grove_short_path() { echo "$1" | sed "s|^$HOME|~|"; }
grove_error() { echo -e "${RED}${1}${RESET}" >&2; }

# ---------- Git helpers ----------
grove_ensure_git() {
    if ! git rev-parse --git-dir &>/dev/null; then
        grove_error "grove: not a git repository"
        return 1
    fi
}

grove_project_name() {
    local remote_url
    remote_url=$(git remote get-url origin 2>/dev/null) || {
        grove_error "grove: no origin remote"
        return 1
    }
    basename "$remote_url" .git
}

# Compute worktree path for a branch
grove_worktree_path() {
    local branch="$1"
    local project_name
    project_name=$(grove_project_name) || return 1
    local safe_branch="${branch//\//-}"
    echo "${GROVE_WORKTREE_BASE}/${project_name}/${safe_branch}"
}

# ---------- Rich status (B1) ----------
# Output: staged modified untracked ahead behind
grove_worktree_status() {
    local dir="$1"
    local status_output
    status_output=$(git -C "$dir" status --porcelain=v2 --branch 2>/dev/null) || {
        echo "0 0 0 0 0"
        return
    }

    local staged=0 modified=0 untracked=0 ahead=0 behind=0

    while IFS= read -r line; do
        case "$line" in
            '# branch.ab '*)
                ahead="${line#*+}";  ahead="${ahead%% *}"
                behind="${line#*-}"; behind="${behind%% *}"
                ;;
            [12]\ *)
                local xy="${line:2:2}"
                [[ "${xy:0:1}" != "." ]] && (( staged++ ))
                [[ "${xy:1:1}" != "." ]] && (( modified++ ))
                ;;
            '? '*)
                (( untracked++ ))
                ;;
        esac
    done <<< "$status_output"

    echo "$staged $modified $untracked $ahead $behind"
}

# Format status for human display (colored)
grove_format_status() {
    local staged=$1 modified=$2 untracked=$3 ahead=$4 behind=$5
    local parts=()

    if (( staged == 0 && modified == 0 && untracked == 0 && ahead == 0 && behind == 0 )); then
        echo -e "${GREEN}clean${RESET}"
        return
    fi

    (( staged > 0 ))    && parts+=("${GREEN}+${staged}${RESET}")
    (( modified > 0 ))  && parts+=("${YELLOW}~${modified}${RESET}")
    (( untracked > 0 )) && parts+=("${RED}?${untracked}${RESET}")
    (( ahead > 0 ))     && parts+=("${CYAN}${ahead}${RESET}")
    (( behind > 0 ))    && parts+=("${MAGENTA}${behind}${RESET}")

    local IFS=' '
    echo -e "${parts[*]}"
}

# ---------- Main worktree ----------
grove_main_worktree_dir() {
    git worktree list | head -1 | awk '{print $1}'
}

# ---------- CD directive ----------
grove_emit_cd() {
    local dir="$1"
    if [[ "$GROVE_PLAIN" == true ]]; then
        echo "$dir"
    else
        echo "__GROVE_CD__:${dir}"
    fi
}

# ---------- Worktree listing helpers ----------
# Parse `git worktree list --porcelain` into structured data
# Sets arrays: _grove_wt_dirs, _grove_wt_branches, _grove_wt_commits
grove_parse_worktrees() {
    _grove_wt_dirs=()
    _grove_wt_branches=()
    _grove_wt_commits=()

    local dir="" branch="" commit=""
    while IFS= read -r line; do
        case "$line" in
            'worktree '*)
                dir="${line#worktree }"
                ;;
            'HEAD '*)
                commit="${line#HEAD }"
                commit="${commit:0:7}"
                ;;
            'branch '*)
                branch="${line#branch refs/heads/}"
                ;;
            'detached')
                branch="(detached)"
                ;;
            '')
                if [[ -n "$dir" ]]; then
                    _grove_wt_dirs+=("$dir")
                    _grove_wt_branches+=("${branch:-???}")
                    _grove_wt_commits+=("${commit:-???????}")
                fi
                dir="" branch="" commit=""
                ;;
        esac
    done < <(git worktree list --porcelain; echo "")
}

# Find worktree dir by branch name, returns 0 on found, 1 on not found
grove_find_by_branch() {
    local target="$1"
    grove_parse_worktrees
    for i in "${!_grove_wt_branches[@]}"; do
        if [[ "${_grove_wt_branches[$i]}" == "$target" ]]; then
            echo "${_grove_wt_dirs[$i]}"
            return 0
        fi
    done
    return 1
}
