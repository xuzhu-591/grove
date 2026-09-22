# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1]

### Improvements

- Make `prune` previews show removal candidates first and group skipped worktrees by reason; add `--verbose` / `-v` for individual skip details.
- Align human-readable tables, shorten home paths and shared directory prefixes, and use stacked, wrapped rows in narrow terminals without truncating names or paths.
- Highlight removal results and always show failed worktrees with complete errors; distinguish unattempted candidates after interrupted cleanup.
- Preserve plain TSV output, cleanup eligibility, confirmation behavior, and existing list behavior. Update Zsh completion for the new flag.

## [0.2.0]

### Features

- Add `grove prune` to preview and remove merged, clean worktrees after refreshing the main worktree. Supports `--dry-run`, `--yes`, and plain TSV output.
- Preserve branches, the current and main worktrees, locked/detached/dirty worktrees, and worktrees with ongoing Git operations. Ignored files are removed with eligible worktrees; external symlink targets are retained.
- Revalidate the base, worktree identity, HEAD, and status before removal; report each skipped or failed item.

### Bug Fixes

- Parse Git porcelain v2 status correctly, including staged/unstaged changes, renames, conflicts, and unusual filenames.
- Show `N/A` and diagnostics when status or merge checks fail instead of claiming clean or unmerged.
- Display the updated commit after automatically fast-forwarding the main worktree; reject missing upstreams as a refresh failure.
- Check real commit references and Git exit status in `remove`'s unpushed-commit check.
- Bind integration tests to Cargo's current test binary instead of potentially stale build artifacts.

## [0.1.7]

### Performance

- `grove list` runs per-worktree status checks concurrently and reuses a single worktree-list parse, cutting runtime roughly 3x on repos with many worktrees.

## [0.1.6]

### Features

- `grove list` refreshes remote refs and fast-forwards a clean, non-diverged main worktree.
- `grove list` warns without updating when the main worktree is dirty, diverged, or remote refresh fails.

## [0.1.5]

### Features

- `grove list` shows merge status of non-main branches (merged/unmerged/-) against the main worktree's branch.

## [0.1.4]

### Bug Fixes

- Allow `grove remove` to clean up prunable worktrees.

## [0.1.3]

### Features

- Unified CI workflow with automated release and changelog
  - Merge ci.yml + release.yml into single ci.yml
  - Add cargo-audit security check
  - Add Swatinem/rust-cache for faster CI builds
  - Release job extracts CHANGELOG.md section and creates GitHub Release
- Add CHANGELOG.md with full project history

### Bug Fixes

- Fix crates.io secret name in publish job (CARGO_TOKEN)

## [0.1.2]

### Bug Fixes

- Project-wide cleanup: i18n, CI, release workflow, config tests
  - Translate all Chinese prompts and docs to English
  - Fix README stale `install.sh` reference
  - Eliminate `env::current_dir()` in core library (accept `cwd` parameter)
  - Fix config tests reading real global config (use isolated temp dirs)
  - Harden release workflow: remove `|| true` masking, retry loop for crates.io index
  - Add E2E smoke test job to CI (ubuntu + macOS)

### Misc

- Bump version to 0.1.2
- Remove install.sh, replaced by cargo install
- Update Cargo.lock

## [0.1.1]

### Bug Fixes

- Use captured cwd in `cmd_remove` to avoid error when worktree directory is deleted
- Fix install command in README: `cargo install grove-cli`
- Add shell integration install instructions for cargo users

### Misc

- Bump version to 0.1.1
- Clean up stray files, track Cargo.lock

## [0.1.0]

Initial release of grove rewritten in Rust.

### Features

- `grove list` — Rich status display (staged, modified, untracked, ahead/behind)
- `grove add` — Create worktree from existing/new/remote branch, auto link cache
- `grove switch` — Jump to worktree with cd support via shell integration
- `grove remove` — Safe removal with uncommitted/unpushed checks
- `grove cache` — Manage build cache symlinks with gitignore-style rules
- Dual output mode: interactive (colored, inquire) and plain (TSV, machine-parseable)
- Zsh shell integration with cd bridging and tab completion
- TOML configuration (`~/.config/grove/config.toml` + `<repo>/grove.toml`)
- Gitignore-style glob pattern matching for cache rules
- Short aliases: `wls`, `wnw`, `wcd`, `wrm`

### Bug Fixes

- Allow grove-core publish to be skipped if already exists on crates.io
- Add version to grove-core dependency for crates.io publish
- Exclude `.claude` worktrees from git
- Rename crate to `grove-cli` for crates.io publish (name `grove` was taken)
- Reuse existing local branch when adding worktree from remote
- Align branch/dir columns in fzf picker for switch and remove
- Correct column alignment and HOME→~ path display
- Widen DIR column cap and improve path truncation
