# Changelog

All notable changes to this project will be documented in this file.

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
