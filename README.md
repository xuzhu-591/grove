# grove

Git worktree manager — interactive for humans, machine-readable for AI/scripts.

## Features

- **`grove list`** — Refresh remote refs, safely fast-forward the main worktree, and show staged, modified, untracked, ahead/behind, and merge state
- **`grove add`** — Create worktree from existing/new/remote branch, auto link cache
- **`grove switch`** — Jump to a worktree (cd support via shell integration)
- **`grove remove`** — Safe removal with uncommitted/unpushed checks
- **`grove prune`** — Preview or remove merged, clean worktrees while retaining branches
- **`grove cache`** — Manage build cache symlinks with gitignore-style rules

Every command supports two modes:

| Mode | When | Output |
|------|------|--------|
| **Human** (default) | Interactive terminal use | Colored, inquire selection |
| **Plain** (`--plain`) | AI agents / scripts | TSV, machine-parseable |

## Install

### Via cargo

```bash
cargo install grove-cli
```

### Shell integration (optional, enables cd + tab completion)

```bash
# Download grove.zsh
mkdir -p ~/.config/grove
curl -fsSL -o ~/.config/grove/grove.zsh \
  https://raw.githubusercontent.com/xuzhu-591/grove/main/shell/grove.zsh

# Add to .zshrc
echo 'source ~/.config/grove/grove.zsh' >> ~/.zshrc
source ~/.zshrc
```

Without shell integration, all commands work normally except `grove switch`/`grove cd` won't change your working directory (they print the path instead).

### From source

```bash
git clone https://github.com/xuzhu-591/grove.git
cd grove
cargo install --path crates/grove
```

## Usage

### Interactive (default)

```bash
grove list              # show all worktrees with status
grove add               # create worktree (interactive branch picker)
grove switch            # jump to worktree (interactive selector)
grove remove            # remove worktree (interactive + safety)
grove prune             # preview and confirm cleanup of merged worktrees
```

### Plain mode (AI / scripts)

```bash
grove --plain list
grove --plain add <branch> [--create] [--remote] [--no-cache]
grove --plain switch <branch>
grove --plain remove <branch> [--force]
grove --plain cache [link|status|unlink]
grove --plain prune --dry-run
grove --plain prune --yes
```

### Plain output format

`grove --plain list` outputs TSV:

```
branch	/path/to/worktree	commit	staged=N	modified=N	untracked=N	ahead=N	behind=N	merged=yes|no|-
```

### Clean up merged worktrees

| Command | Behavior |
|---------|----------|
| `grove prune` | Show candidates and skipped worktrees, then confirm removal |
| `grove prune --dry-run` | Show candidates without deleting worktrees |
| `grove prune --yes` | Remove eligible worktrees without a confirmation prompt |
| `grove --plain prune --dry-run` | Preview in TSV format |
| `grove --plain prune --yes` | Remove and report results in TSV format |

Prune refreshes remote refs and safely fast-forwards the main worktree before evaluating candidates, just like `list`. Even `--dry-run` can update the main worktree; it only disables deletion. A failed refresh, missing upstream, dirty main worktree that needs updating, or diverged main branch stops cleanup.

A branch is considered merged when its HEAD is an ancestor of (or equal to) the main worktree's current branch commit. This is a local Git ancestry check, not a pull-request status check: squash/rebase merges may not qualify, and commits present only on the local main branch can qualify. The exact base branch and full commit are printed before cleanup.

Prune skips the main/current worktree, locked or detached worktrees, missing or invalid worktrees, unmerged branches, worktrees with Git operations in progress, and worktrees with staged, modified, conflicting, or untracked files. It also skips a worktree that contains another registered worktree. Candidate identity, HEAD, status, and the base are rechecked before removal.

**Ignored files inside an eligible worktree are deleted too**, including local configuration, dependencies, logs, and build output. Symlinks are removed without following them to delete external targets. Local and remote branches are retained. Git's non-force worktree removal is used; refusals such as initialized submodules are reported as failures. There is no force option.

Non-interactive use requires `--dry-run` or `--yes`. Plain output is headerless TSV:

```text
branch\t/path/to/worktree\tcandidate|skipped|removed|failed\treason
```

Backslashes, tabs, newlines, and carriage returns in prune fields are escaped as `\\`, `\t`, `\n`, and `\r`. Diagnostics and the summary go to stderr. Skipped items are expected and do not cause an error exit; removal failures or an invalid/changed base return nonzero. If the base changes after some removals, completed results are reported and remaining `candidate` rows were not attempted.

`grove prune` removes actual worktree directories. Git's separate `git worktree prune` command only cleans stale administrative entries.

### Status and removal checks

`list` still refreshes automatically and keeps its existing parallel status checks. Failed status fields or an unavailable merge comparison are shown as `N/A`, with diagnostics on stderr. The existing TSV field order is unchanged; scripts should accept `N/A` as an unknown value.

`remove` checks commits against the branch's configured upstream, or the main worktree commit when no upstream is configured. Failed checks stop removal unless the existing `--force` option is explicitly used.

## Configuration

### Worktree base path

| Priority | Source |
|----------|--------|
| 1 (highest) | `GROVE_WORKTREE_BASE` env var |
| 2 | `config.worktree.base_path` in config file |
| 3 (default) | `~/.grove/worktrees` |

### Cache rules

Define directories to symlink from the main worktree into new ones:

```toml
# ~/.config/grove/config.toml (global, all projects)
[cache]
rules = [
    "node_modules",
    ".cache/*",
]

[worktree]
# base_path = "~/worktrees"
```

```toml
# <project>/grove.toml (project-specific, overrides global)
[cache]
rules = [
    "!**/test",
    "packages/*/node_modules",
]
```

Rules use a gitignore subset: literal paths, `*`, `?`, `**`, `!negation`, `/anchored`. Evaluated last-match-wins across both config files.

## Requirements

- Rust toolchain (for install from source)
- Git
- Zsh (for shell integration)

## License

[MIT](LICENSE)
