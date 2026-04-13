# grove

Git worktree manager with dual-mode support — interactive (fzf) for humans, plain for AI/scripts.

## Features

- **`grove list`** — Rich status display: staged, modified, untracked, ahead/behind
- **`grove add`** — Create worktree from existing or new branch
- **`grove switch`** — Jump to a worktree (with log preview)
- **`grove remove`** — Safe removal with uncommitted/unpushed checks

Every command supports two modes:

| Mode | When | Output |
|------|------|--------|
| **fzf** (default) | Interactive terminal use | Colored, fzf selection, preview |
| **plain** (`--plain`) | AI agents / scripts | TSV, machine-parseable |

## Install

```bash
git clone https://github.com/AmazoniteC/grove.git
cd grove
bash install.sh
source ~/.zshrc
```

The installer symlinks `grove` to `~/.local/bin/` and adds shell integration to `.zshrc`.

## Usage

### Interactive (human)

```bash
grove list              # show all worktrees with status
grove add               # create worktree (fzf branch picker)
grove switch            # jump to worktree (fzf selector + log preview)
grove remove            # remove worktree (fzf + safety checks)
```

### Plain mode (AI / scripts)

```bash
grove --plain list
grove --plain add <branch> [--create]
grove --plain switch <branch>
grove --plain remove <branch> [--force]
```

### Plain output format

`grove --plain list` outputs TSV:

```
branch	/path/to/worktree	commit	staged=N	modified=N	untracked=N	ahead=N	behind=N
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `GROVE_WORKTREE_BASE` | `~/.grove/worktrees` | Base directory for worktrees |

Worktrees are organized as `{base}/{project}/{branch}`.

## Requirements

- Bash 4+
- Git
- [fzf](https://github.com/junegunn/fzf) (for interactive mode)
- Zsh (for shell integration / tab completion)

## License

[MIT](LICENSE)
