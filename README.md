# grove

Git worktree manager — interactive for humans, machine-readable for AI/scripts.

## Features

- **`grove list`** — Refresh remote refs, safely fast-forward the main worktree, and show staged, modified, untracked, ahead/behind, and merge state
- **`grove add`** — Create worktree from existing/new/remote branch, auto link cache
- **`grove switch`** — Jump to a worktree (cd support via shell integration)
- **`grove remove`** — Safe removal with uncommitted/unpushed checks
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
```

### Plain mode (AI / scripts)

```bash
grove --plain list
grove --plain add <branch> [--create] [--remote] [--no-cache]
grove --plain switch <branch>
grove --plain remove <branch> [--force]
grove --plain cache [link|status|unlink]
```

### Plain output format

`grove --plain list` outputs TSV:

```
branch	/path/to/worktree	commit	staged=N	modified=N	untracked=N	ahead=N	behind=N	merged=yes|no|-
```

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
