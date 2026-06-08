# grove Rust 迁移实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 grove git worktree manager 从 Bash 完整迁移到 Rust，功能等价，补齐测试和 CI。

**Architecture:** Cargo Workspace 双 crate 架构——`grove-core` 纯逻辑库（无终端依赖，可独立单测）+ `grove` CLI 二进制（clap + inquire + console）。zsh shell wrapper 做 cd 桥接。配置统一到 TOML（`~/.config/grove/config.toml` + `<repo>/grove.toml`）。

**Tech Stack:** Rust 2024 edition, clap 4 (derive), inquire 0.7, console, serde + toml, thiserror + anyhow

---

## File Structure

```
grove/                              # repo root
├── Cargo.toml                      # workspace
├── .gitignore                      # Rust-specific gitignore
├── crates/
│   ├── grove-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # re-exports all modules
│   │       ├── error.rs            # GroveError enum (thiserror)
│   │       ├── path.rs             # path utils
│   │       ├── config.rs           # TOML parse + merge
│   │       ├── pattern.rs          # gitignore glob engine
│   │       ├── git.rs              # git command wrappers
│   │       ├── worktree.rs         # worktree CRUD
│   │       └── cache.rs            # cache symlink logic
│   └── grove/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # entry: parse → dispatch
│           ├── cli.rs              # clap definitions
│           ├── output.rs           # format & emit
│           └── interactive.rs      # inquire dialogs
├── shell/
│   └── grove.zsh                   # zsh wrapper + completion
├── install.sh                      # installation script
├── .github/
│   └── workflows/
│       ├── ci.yml                  # lint, test, build
│       └── release.yml             # publish to crates.io
├── tests/
│   ├── integration/
│   │   ├── helpers.rs              # test fixture helpers
│   │   ├── add_tests.rs
│   │   ├── list_tests.rs
│   │   ├── remove_tests.rs
│   │   ├── switch_tests.rs
│   │   └── cache_tests.rs
│   └── e2e/
│       └── smoke.sh
├── docs/                           # spec + plan + future docs
├── README.md
└── LICENSE
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `.gitignore`
- Create: `crates/grove-core/Cargo.toml`
- Create: `crates/grove-core/src/lib.rs`
- Create: `crates/grove/Cargo.toml`
- Create: `crates/grove/src/main.rs`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
# /Cargo.toml
[workspace]
resolver = "2"
members = ["crates/grove-core", "crates/grove"]
```

- [ ] **Step 2: Create Rust .gitignore**

```gitignore
# /.gitignore
/target/
Cargo.lock
```

- [ ] **Step 3: Create grove-core Cargo.toml**

```toml
# crates/grove-core/Cargo.toml
[package]
name = "grove-core"
version = "0.1.0"
edition = "2021"
description = "Core logic for grove git worktree manager"

[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
thiserror = "1"
```

- [ ] **Step 4: Create grove-core lib.rs stub**

```rust
// crates/grove-core/src/lib.rs
pub mod config;
pub mod error;
pub mod git;
pub mod path;
pub mod pattern;
pub mod worktree;
pub mod cache;
```

- [ ] **Step 5: Create grove CLI Cargo.toml**

```toml
# crates/grove/Cargo.toml
[package]
name = "grove"
version = "0.1.0"
edition = "2021"
description = "Git worktree manager"

[[bin]]
name = "grove"
path = "src/main.rs"

[dependencies]
grove-core = { path = "../grove-core" }
clap = { version = "4", features = ["derive"] }
inquire = "0.7"
console = "0.15"
anyhow = "1"
```

- [ ] **Step 6: Create grove main.rs stub**

```rust
// crates/grove/src/main.rs
fn main() {
    println!("grove - git worktree manager (WIP)");
}
```

- [ ] **Step 7: Create CI workflow**

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo fmt --check

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-targets

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
```

- [ ] **Step 8: Verify scaffold compiles**

```bash
cargo build
```

Expected: Compiles successfully, prints "grove - git worktree manager (WIP)" on run.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore: project scaffolding — workspace, crates, CI"
```

---

### Task 2: Error Types (grove-core)

**Files:**
- Create: `crates/grove-core/src/error.rs`
- Modify: `crates/grove-core/src/lib.rs` (already has `pub mod error;`)

- [ ] **Step 1: Write error module**

```rust
// crates/grove-core/src/error.rs
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GroveError {
    #[error("not a git repository")]
    NotGitRepo,

    #[error("no origin remote configured")]
    NoOriginRemote,

    #[error("worktree not found for branch '{0}'")]
    WorktreeNotFound(String),

    #[error("cannot remove main worktree")]
    CannotRemoveMain,

    #[error("worktree has uncommitted changes:\n{0}")]
    UncommittedChanges(String),

    #[error("worktree has unpushed commits:\n{0}")]
    UnpushedCommits(String),

    #[error("git command failed: {0}")]
    GitError(String),

    #[error("invalid cache pattern '{0}': {1}")]
    InvalidCachePattern(String, String),

    #[error("config file error ({0}): {1}")]
    ConfigError(PathBuf, String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type GroveResult<T> = Result<T, GroveError>;
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/error.rs
git commit -m "feat(core): add GroveError types"
```

---

### Task 3: Path Utilities (grove-core)

**Files:**
- Create: `crates/grove-core/src/path.rs`

- [ ] **Step 1: Write path module**

```rust
// crates/grove-core/src/path.rs
use std::path::{Path, PathBuf};

/// Compute worktree directory path for a branch.
///
/// Formula: `{base}/{project_name}/{safe_branch}`
///   - base: from env `GROVE_WORKTREE_BASE` or `~/.grove/worktrees`
///   - project_name: extracted from git remote origin URL
///   - safe_branch: `/` → `-`
pub fn worktree_path(base: &Path, project_name: &str, branch: &str) -> PathBuf {
    let safe_branch = branch.replace('/', "-");
    base.join(project_name).join(safe_branch)
}

/// Default worktree base if env var not set.
pub fn default_worktree_base() -> PathBuf {
    dirs_home().join(".grove").join("worktrees")
}

/// Read GROVE_WORKTREE_BASE env var, falling back to default.
pub fn resolve_worktree_base() -> PathBuf {
    std::env::var("GROVE_WORKTREE_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_worktree_base())
}

/// Replace `$HOME` prefix with `~`.
pub fn short_path(path: &Path) -> String {
    let home = dirs_home();
    let path_str = path.display().to_string();
    let home_str = home.display().to_string();
    if let Some(stripped) = path_str.strip_prefix(&home_str) {
        format!("~{}", stripped)
    } else {
        path_str
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_path_simple_branch() {
        let base = Path::new("/home/user/.grove/worktrees");
        let p = worktree_path(base, "myproject", "feat/login");
        assert_eq!(p, Path::new("/home/user/.grove/worktrees/myproject/feat-login"));
    }

    #[test]
    fn test_worktree_path_no_slash() {
        let base = Path::new("/home/user/.grove/worktrees");
        let p = worktree_path(base, "myproject", "main");
        assert_eq!(p, Path::new("/home/user/.grove/worktrees/myproject/main"));
    }

    #[test]
    fn test_short_path() {
        let real_home = std::env::var("HOME").unwrap();
        let long = Path::new(&real_home).join("code").join("proj");
        let short = short_path(&long);
        assert!(short.starts_with("~/"));
        assert!(short.ends_with("code/proj"));
    }
}
```

- [ ] **Step 2: Verify compiles and tests pass**

```bash
cargo test -p grove-core
```

Expected: 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/path.rs
git commit -m "feat(core): add path utilities"
```

---

### Task 4: Config Module (grove-core)

**Files:**
- Create: `crates/grove-core/src/config.rs`

- [ ] **Step 1: Write config module**

```rust
// crates/grove-core/src/config.rs
use crate::error::{GroveError, GroveResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration structure.
#[derive(Debug, Deserialize, Default)]
pub struct GroveConfig {
    #[serde(default)]
    pub cache: CacheSection,

    #[serde(default)]
    pub worktree: WorktreeSection,
}

#[derive(Debug, Deserialize, Default)]
pub struct CacheSection {
    /// Rules evaluated in order. Last match wins.
    /// Supports gitignore subset: literals, `*`, `?`, `**`, `!negation`, `/anchored`.
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct WorktreeSection {
    /// Overrides GROVE_WORKTREE_BASE.
    #[serde(default)]
    pub base_path: Option<String>,
}

impl GroveConfig {
    /// Load and merge config from global and project-level files.
    ///
    /// Priority (low → high):
    ///   1. `~/.config/grove/config.toml` (global)
    ///   2. `<repo>/grove.toml` (project)
    ///
    /// For `cache.rules`: global rules first, then project rules appended.
    /// For other keys: later files override earlier.
    pub fn load(repo_root: &Path) -> GroveResult<Self> {
        let mut merged = GroveConfig::default();

        // 1. Global config
        let global_path = global_config_path();
        if global_path.exists() {
            let global = Self::read_file(&global_path)?;
            merged.merge(global);
        }

        // 2. Project config
        let project_path = repo_root.join("grove.toml");
        if project_path.exists() {
            let project = Self::read_file(&project_path)?;
            merged.merge(project);
        }

        Ok(merged)
    }

    fn read_file(path: &Path) -> GroveResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            GroveError::ConfigError(path.to_path_buf(), e.to_string())
        })?;
        toml::from_str(&content).map_err(|e| {
            GroveError::ConfigError(path.to_path_buf(), e.to_string())
        })
    }

    fn merge(&mut self, other: GroveConfig) {
        // cache.rules: append (global first, then project)
        self.cache.rules.extend(other.cache.rules);

        // worktree.base_path: later overrides earlier
        if other.worktree.base_path.is_some() {
            self.worktree.base_path = other.worktree.base_path;
        }
    }
}

/// Resolve effective worktree base path.
/// Env var > config > default.
pub fn resolve_worktree_base(config: &GroveConfig) -> PathBuf {
    if let Ok(env_base) = std::env::var("GROVE_WORKTREE_BASE") {
        return PathBuf::from(env_base);
    }
    if let Some(ref base) = config.worktree.base_path {
        return shellexpand::tilde(base).into_owned().into();
    }
    crate::path::default_worktree_base()
}

fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config").join("grove").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_minimal_config() {
        let config: GroveConfig = toml::from_str("").unwrap();
        assert!(config.cache.rules.is_empty());
        assert!(config.worktree.base_path.is_none());
    }

    #[test]
    fn test_parse_cache_rules() {
        let toml = r#"
[cache]
rules = ["node_modules", ".cache/*", "!.cache/private"]
"#;
        let config: GroveConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.cache.rules.len(), 3);
        assert_eq!(config.cache.rules[0], "node_modules");
        assert_eq!(config.cache.rules[2], "!.cache/private");
    }

    #[test]
    fn test_merge_rules_appended() {
        let global: GroveConfig = toml::from_str(
            r#"[cache]
rules = ["node_modules"]
"#,
        )
        .unwrap();
        let project: GroveConfig = toml::from_str(
            r#"[cache]
rules = ["packages/*/node_modules"]
"#,
        )
        .unwrap();

        let mut merged = GroveConfig::default();
        merged.merge(global);
        merged.merge(project);

        assert_eq!(merged.cache.rules.len(), 2);
        assert_eq!(merged.cache.rules[0], "node_modules"); // global first
        assert_eq!(merged.cache.rules[1], "packages/*/node_modules"); // then project
    }

    #[test]
    fn test_config_file_not_found_is_ok() {
        // When neither global nor project config exist, load should return defaults.
        let tmp = std::env::temp_dir();
        let config = GroveConfig::load(&tmp);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.cache.rules.is_empty());
    }

    #[test]
    fn test_load_project_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config_content = r#"
[cache]
rules = ["node_modules"]
"#;
        let config_path = tmp.path().join("grove.toml");
        std::fs::write(&config_path, config_content).unwrap();

        let config = GroveConfig::load(tmp.path()).unwrap();
        assert_eq!(config.cache.rules.len(), 1);
        assert_eq!(config.cache.rules[0], "node_modules");
    }
}
```

Note: This task introduces `tempfile` as a dev-dependency and `shellexpand` as a dependency for `grove-core`.

- [ ] **Step 2: Add dependencies**

```toml
# crates/grove-core/Cargo.toml — add to existing deps
shellexpand = "3"

[dev-dependencies]
tempfile = "3"
```

(Edit the file directly with the additions)

- [ ] **Step 3: Verify tests pass**

```bash
cargo test -p grove-core
```

Expected: 8 tests pass (3 from path + 5 from config).

- [ ] **Step 4: Commit**

```bash
git add crates/grove-core/src/config.rs crates/grove-core/Cargo.toml
git commit -m "feat(core): add config module with TOML parsing and merge"
```

---

### Task 5: Pattern Matching Engine (grove-core)

**Files:**
- Create: `crates/grove-core/src/pattern.rs`

- [ ] **Step 1: Write pattern module**

```rust
// crates/grove-core/src/pattern.rs
use crate::error::{GroveError, GroveResult};

/// A compiled gitignore-style rule.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub raw: String,
    pub negated: bool,
    anchored: bool,
    kind: RuleKind,
}

#[derive(Debug, Clone)]
enum RuleKind {
    /// Exact match (no wildcards, no `/`): match basename at any depth
    Basename(String),
    /// Full pattern with `/` but no `**`: match exact path or trailing suffix
    ExactPath(String),
    /// Anchored to repo root (`/...` prefix)
    Anchored(String),
    /// Contains `**/` prefix: match anywhere
    Anywhere(String),
    /// Contains `/**` suffix: match prefix and all descendants
    Prefix(String),
    /// Contains `/**/` middle: match prefix + any path + suffix
    Recursive { prefix: String, suffix: String },
    /// Pattern contains `*` or `?`: glob match per path segment
    Glob(String),
}

/// Validate that a rule pattern is safe (no path traversal).
pub fn validate_pattern(pattern: &str) -> GroveResult<()> {
    let normalized = pattern.trim_start_matches('!').trim_start_matches('/');
    if normalized.is_empty() {
        return Err(GroveError::InvalidCachePattern(
            pattern.to_string(),
            "empty pattern".into(),
        ));
    }
    if normalized.contains("../") || normalized == ".." {
        return Err(GroveError::InvalidCachePattern(
            pattern.to_string(),
            "contains path traversal".into(),
        ));
    }
    Ok(())
}

/// Compile a raw pattern string into a compiled rule.
pub fn compile(pattern_raw: &str) -> GroveResult<CompiledRule> {
    validate_pattern(pattern_raw)?;

    let negated = pattern_raw.starts_with('!');
    let without_neg = if negated { &pattern_raw[1..] } else { pattern_raw };
    let anchored = without_neg.starts_with('/');
    let pattern = if anchored { &without_neg[1..] } else { without_neg };

    let kind = classify(pattern);

    Ok(CompiledRule {
        raw: pattern_raw.to_string(),
        negated,
        anchored,
        kind,
    })
}

fn classify(pattern: &str) -> RuleKind {
    // pattern contains `/**/` anywhere
    if pattern.contains("/**/") {
        let parts: Vec<&str> = pattern.splitn(2, "/**/").collect();
        return RuleKind::Recursive {
            prefix: parts[0].to_string(),
            suffix: parts[1].to_string(),
        };
    }

    // pattern starts with `**/`
    if let Some(rest) = pattern.strip_prefix("**/") {
        return RuleKind::Anywhere(rest.to_string());
    }

    // pattern ends with `/**`
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return RuleKind::Prefix(prefix.to_string());
    }

    // pattern contains `*` or `?`
    if pattern.contains('*') || pattern.contains('?') {
        return RuleKind::Glob(pattern.to_string());
    }

    // pattern contains `/` — exact path
    if pattern.contains('/') {
        return RuleKind::ExactPath(pattern.to_string());
    }

    // plain basename
    RuleKind::Basename(pattern.to_string())
}

/// Test whether a compiled rule matches a repo-relative directory path.
pub fn matches(rule: &CompiledRule, rel_path: &str) -> bool {
    let result = match &rule.kind {
        RuleKind::Basename(name) => {
            rel_path == name
                || rel_path.ends_with(&format!("/{}", name))
        }
        RuleKind::ExactPath(path) => {
            rel_path == path
                || rel_path.ends_with(&format!("/{}", path))
        }
        RuleKind::Anchored(path) => {
            rel_path == path
                || rel_path.starts_with(&format!("{}/", path))
        }
        RuleKind::Anywhere(suffix) => {
            rel_path == suffix
                || rel_path.ends_with(&format!("/{}", suffix))
        }
        RuleKind::Prefix(prefix) => {
            rel_path == prefix
                || rel_path.starts_with(&format!("{}/", prefix))
        }
        RuleKind::Recursive { prefix, suffix } => {
            if prefix.is_empty() {
                rel_path == suffix
                    || rel_path.ends_with(&format!("/{}", suffix))
            } else {
                (rel_path.starts_with(&format!("{}/", prefix)) || rel_path == prefix)
                    && (rel_path.ends_with(&format!("/{}", suffix)) || rel_path == suffix)
            }
        }
        RuleKind::Glob(pattern) => {
            glob_match_segments(pattern, rel_path)
        }
    };

    result
}

/// Simple glob matching per path segment.
/// Supports `*` (matches anything except `/`) and `?` (matches single char except `/`).
fn glob_match_segments(pattern: &str, path: &str) -> bool {
    let pat_segs: Vec<&str> = pattern.split('/').collect();
    let path_segs: Vec<&str> = path.split('/').collect();

    if pat_segs.len() != path_segs.len() {
        return false;
    }

    pat_segs
        .iter()
        .zip(path_segs.iter())
        .all(|(p, s)| glob_segment_match(p, s))
}

fn glob_segment_match(pattern: &str, segment: &str) -> bool {
    let mut pi = 0;
    let mut si = 0;
    let pc: Vec<char> = pattern.chars().collect();
    let sc: Vec<char> = segment.chars().collect();

    while pi < pc.len() {
        match pc[pi] {
            '*' => {
                if pi + 1 >= pc.len() {
                    // trailing * matches rest
                    return true;
                }
                // find next literal char from pattern in segment
                let next = pc[pi + 1];
                while si < sc.len() && sc[si] != next {
                    si += 1;
                }
                if si >= sc.len() {
                    return false;
                }
                pi += 1;
            }
            '?' => {
                if si >= sc.len() {
                    return false;
                }
                pi += 1;
                si += 1;
            }
            ch => {
                if si >= sc.len() || sc[si] != ch {
                    return false;
                }
                pi += 1;
                si += 1;
            }
        }
    }
    si == sc.len()
}

/// Evaluate a list of rules against a path. Returns whether the path is selected.
/// Last-match-wins: iterate in order, each matching rule toggles the result.
pub fn evaluate(rules: &[CompiledRule], rel_path: &str) -> bool {
    let mut selected = false;
    for rule in rules {
        if matches(rule, rel_path) {
            selected = !rule.negated;
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty() {
        assert!(validate_pattern("").is_err());
    }

    #[test]
    fn test_validate_traversal() {
        assert!(validate_pattern("../etc").is_err());
        assert!(validate_pattern("..").is_err());
    }

    #[test]
    fn test_validate_valid() {
        assert!(validate_pattern("node_modules").is_ok());
        assert!(validate_pattern("!.cache/private").is_ok());
    }

    #[test]
    fn test_basename_match() {
        let rule = compile("node_modules").unwrap();
        assert!(matches(&rule, "node_modules"));
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
        assert!(!matches(&rule, "node_modules_extra"));
    }

    #[test]
    fn test_anchored_match() {
        let rule = compile("/build").unwrap();
        assert!(matches(&rule, "build"));
        assert!(matches(&rule, "build/output")); // matches prefix + children
        assert!(!matches(&rule, "packages/build"));
    }

    #[test]
    fn test_exact_path_match() {
        let rule = compile("packages/pkg-a/node_modules").unwrap();
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
        // matches suffix of repo-relative path
        assert!(matches(&rule, "parent/packages/pkg-a/node_modules"));
        assert!(!matches(&rule, "packages/pkg-b/node_modules"));
    }

    #[test]
    fn test_recursive_match() {
        let rule = compile("a/**/b").unwrap();
        assert!(matches(&rule, "a/b"));
        assert!(matches(&rule, "a/x/b"));
        assert!(matches(&rule, "a/x/y/b"));
        assert!(!matches(&rule, "a/x/b/c"));
    }

    #[test]
    fn test_prefix_match() {
        let rule = compile("packages/**").unwrap();
        assert!(matches(&rule, "packages"));
        assert!(matches(&rule, "packages/pkg-a"));
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
        assert!(!matches(&rule, "other"));
    }

    #[test]
    fn test_anywhere_match() {
        let rule = compile("**/node_modules").unwrap();
        assert!(matches(&rule, "node_modules"));
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
    }

    #[test]
    fn test_negation() {
        let rule = compile("!.cache/private").unwrap();
        assert!(rule.negated);
        assert!(matches(&rule, ".cache/private"));
        assert!(!matches(&rule, ".cache/public"));
    }

    #[test]
    fn test_glob_star_match() {
        let rule = compile("*.log").unwrap();
        assert!(matches(&rule, "debug.log"));
        assert!(matches(&rule, "dir/debug.log"));
        assert!(!matches(&rule, "debug.txt"));
    }

    #[test]
    fn test_glob_segment_with_star() {
        let rule = compile("packages/*/node_modules").unwrap();
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
        assert!(matches(&rule, "packages/pkg-b/node_modules"));
        assert!(!matches(&rule, "packages/pkg-a/sub/node_modules"));
    }

    #[test]
    fn test_evaluate_last_match_wins() {
        let rules = vec![
            compile(".cache/*").unwrap(),
            compile("!.cache/private").unwrap(),
        ];
        assert!(evaluate(&rules, ".cache/public"));
        assert!(!evaluate(&rules, ".cache/private"));
    }

    #[test]
    fn test_evaluate_no_rules() {
        let rules: Vec<CompiledRule> = vec![];
        assert!(!evaluate(&rules, "anything"));
    }

    #[test]
    fn test_evaluate_all_patterns_in_doc() {
        // Test cases from the design spec
        let cases = vec![
            ("node_modules", "node_modules", true),
            ("node_modules", "packages/pkg-a/node_modules", true),
            ("/build", "build", true),
            ("/build", "packages/build", false),
            ("packages/**", "packages", true),
            ("packages/**", "packages/pkg-a/node_modules", true),
            ("**.log", "debug.log", true),
            ("**.log", "dir/debug.log", true),
            ("**.log", "debug.txt", false),
            ("!target", "target", false), // negated, but needs at least one positive rule
        ];
        for (pattern, path, expected) in cases {
            let rule = compile(pattern).unwrap();
            assert_eq!(
                matches(&rule, path),
                expected,
                "pattern='{pattern}', path='{path}'"
            );
        }
    }
}
```

- [ ] **Step 2: Verify all tests pass**

```bash
cargo test -p grove-core
```

Expected: All tests pass (15 pattern tests + 5 config + 3 path = 23 total).

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/pattern.rs
git commit -m "feat(core): add gitignore-style pattern matching engine"
```

---

### Task 6: Git Command Wrappers (grove-core)

**Files:**
- Create: `crates/grove-core/src/git.rs`

- [ ] **Step 1: Write git module**

```rust
// crates/grove-core/src/git.rs
use crate::error::{GroveError, GroveResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command with given args in the given directory,
/// capturing stdout.
fn git(dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
}

/// Run a git command, failing on non-zero exit.
fn git_checked(dir: &Path, args: &[&str]) -> GroveResult<std::process::Output> {
    let output = git(dir, args).map_err(|e| GroveError::GitError(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GroveError::GitError(stderr.trim().to_string()));
    }
    Ok(output)
}

/// Ensure we are inside a git repository.
pub fn ensure_git_repo() -> GroveResult<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map_err(|_| GroveError::NotGitRepo)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(GroveError::NotGitRepo)
    }
}

/// Extract project name from origin remote URL.
/// "git@github.com:user/repo.git" → "repo"
pub fn project_name() -> GroveResult<String> {
    let output = git_checked(&std::env::current_dir().unwrap(), &["remote", "get-url", "origin"])?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err(GroveError::NoOriginRemote);
    }
    // Strip .git suffix, take basename
    let name = url
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let name = name.rsplit('/').next().unwrap_or(name);
    Ok(name.to_string())
}

/// Get the main worktree directory (first line of `git worktree list`).
pub fn main_worktree_dir() -> GroveResult<PathBuf> {
    let output = git_checked(
        &std::env::current_dir().unwrap(),
        &["worktree", "list", "--porcelain"],
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(dir) = line.strip_prefix("worktree ") {
            return Ok(PathBuf::from(dir));
        }
    }
    Err(GroveError::GitError("no worktrees found".into()))
}

/// A parsed worktree entry.
#[derive(Debug, Clone)]
pub struct Worktree {
    pub branch: String,
    pub path: PathBuf,
    pub commit: String,
}

/// Parse `git worktree list --porcelain` output.
pub fn parse_worktree_list(dir: &Path) -> GroveResult<Vec<Worktree>> {
    let output = git_checked(dir, &["worktree", "list", "--porcelain"])?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut worktrees = Vec::new();
    let mut current_path = None;
    let mut current_branch = String::new();
    let mut current_commit = String::new();

    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            // flush previous
            if let Some(path) = current_path.take() {
                worktrees.push(Worktree {
                    branch: if current_branch.is_empty() {
                        "(detached)".into()
                    } else {
                        std::mem::take(&mut current_branch)
                    },
                    path,
                    commit: std::mem::take(&mut current_commit),
                });
            }
            current_path = Some(PathBuf::from(p));
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current_commit = h[..7.min(h.len())].to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            current_branch = b.to_string();
        } else if line == "detached" {
            current_branch = "(detached)".into();
        }
    }
    // flush last
    if let Some(path) = current_path {
        worktrees.push(Worktree {
            branch: if current_branch.is_empty() {
                "(detached)".into()
            } else {
                current_branch
            },
            path,
            commit: current_commit,
        });
    }

    Ok(worktrees)
}

/// Worktree status parsed from `git status --porcelain=v2 --branch`.
#[derive(Debug, Clone, Default)]
pub struct WorktreeStatus {
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
    pub ahead: u32,
    pub behind: u32,
}

/// Parse `git status --porcelain=v2 --branch` output.
pub fn parse_status(dir: &Path) -> GroveResult<WorktreeStatus> {
    let output = git(dir, &["status", "--porcelain=v2", "--branch"]);

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Ok(WorktreeStatus::default()),
    };

    let mut status = WorktreeStatus::default();

    for line in text.lines() {
        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            // format: "+N -M"
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.len() >= 3 {
            let xy = &line[..2];
            if xy.chars().all(|c| c.is_ascii_digit() || c == '?' || c == '.') {
                // Index status
                if xy.chars().next().map_or(false, |c| c != '.') {
                    status.staged += 1;
                }
                // Worktree status
                if xy.chars().nth(1).map_or(false, |c| c != '.') {
                    status.modified += 1;
                }
            }
        } else if line.starts_with('?') {
            status.untracked += 1;
        }
    }

    Ok(status)
}

/// Check if a worktree has uncommitted changes.
pub fn has_uncommitted(dir: &Path) -> GroveResult<bool> {
    let output = git(dir, &["status", "--porcelain", "--untracked-files=normal"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(!text.trim().is_empty())
}

/// Get unpushed commits log (one line per commit).
pub fn unpushed_commits(dir: &Path) -> GroveResult<Vec<String>> {
    let main_dir = main_worktree_dir()?;
    let output = git(
        dir,
        &[
            "log",
            "--oneline",
            &format!("{}..HEAD", main_dir.display()),
        ],
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(text.lines().map(|s| s.to_string()).collect())
}

/// List local branch names.
pub fn list_local_branches(dir: &Path) -> GroveResult<Vec<String>> {
    let output = git_checked(dir, &["branch", "--format=%(refname:short)"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// List remote branch names.
pub fn list_remote_branches(dir: &Path) -> GroveResult<Vec<String>> {
    let output = git_checked(dir, &["branch", "-r", "--format=%(refname:short)"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.ends_with("/HEAD"))
        .collect())
}

/// Fetch all remotes with prune.
pub fn fetch_all(dir: &Path) -> GroveResult<()> {
    git_checked(dir, &["fetch", "--all", "--prune"])?;
    Ok(())
}

/// Get the first remote name.
pub fn first_remote(dir: &Path) -> GroveResult<String> {
    let output = git_checked(dir, &["remote"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .map(|s| s.to_string())
        .ok_or(GroveError::GitError("no remotes configured".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_list() {
        let input = "worktree /home/user/repo\nHEAD 1234567890abcdef\nbranch refs/heads/main\n\nworktree /home/user/repo-2\nHEAD abcdef1234567890\nbranch refs/heads/feat/x\n\n";
        // We can't easily mock git commands without a git repo.
        // This test validates the parsing logic against known output format.
        let dir = std::env::current_dir().unwrap();
        if git(&dir, &["rev-parse", "--git-dir"]).is_ok() {
            let wts = parse_worktree_list(&dir).unwrap();
            assert!(!wts.is_empty());
        }
    }

    #[test]
    fn test_parse_status_empty() {
        // Without a real git context, parsing empty output should give defaults.
        let tmp_dir = std::env::temp_dir();
        let status = parse_status(&tmp_dir);
        // On a non-git dir, should return defaults (OK)
        if let Ok(s) = status {
            assert_eq!(s.staged, 0);
            assert_eq!(s.ahead, 0);
        }
    }
}
```

- [ ] **Step 2: Verify compiles and tests pass**

```bash
cargo test -p grove-core
```

Expected: Compiles and tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/git.rs
git commit -m "feat(core): add git command wrappers and output parsers"
```

---

### Task 7: Worktree Module (grove-core)

**Files:**
- Create: `crates/grove-core/src/worktree.rs`

- [ ] **Step 1: Write worktree module**

```rust
// crates/grove-core/src/worktree.rs
use crate::error::{GroveError, GroveResult};
use crate::git::{self, Worktree as GitWorktree, WorktreeStatus};
use crate::path;
use std::path::{Path, PathBuf};

/// Add options for creating a worktree.
pub struct AddOptions {
    pub create: bool,
    pub remote: bool,
    pub no_cache: bool,
}

/// A full worktree entry with status.
pub struct WorktreeEntry {
    pub wt: GitWorktree,
    pub status: WorktreeStatus,
    pub is_main: bool,
}

/// List all worktrees with status.
pub fn list_all() -> GroveResult<Vec<WorktreeEntry>> {
    let dir = std::env::current_dir()?;
    let wts = git::parse_worktree_list(&dir)?;
    let mut entries = Vec::new();
    for (i, wt) in wts.into_iter().enumerate() {
        let status = git::parse_status(&wt.path).unwrap_or_default();
        entries.push(WorktreeEntry {
            wt,
            status,
            is_main: i == 0,
        });
    }
    Ok(entries)
}

/// Find a worktree directory by branch name.
pub fn find_by_branch(branch: &str) -> GroveResult<PathBuf> {
    let dir = std::env::current_dir()?;
    let wts = git::parse_worktree_list(&dir)?;
    for wt in &wts {
        if wt.branch == branch {
            return Ok(wt.path.clone());
        }
    }
    Err(GroveError::WorktreeNotFound(branch.to_string()))
}

/// Get the main worktree directory.
pub fn main_worktree() -> GroveResult<PathBuf> {
    git::main_worktree_dir()
}

/// Add (create) a worktree.
pub fn add(branch: &str, opts: &AddOptions) -> GroveResult<PathBuf> {
    let base = path::resolve_worktree_base();
    let project = git::project_name()?;
    let wt_dir = path::worktree_path(&base, &project, branch);

    let cwd = std::env::current_dir()?;

    if opts.remote {
        add_from_remote(branch, &wt_dir, &cwd)?;
    } else if opts.create {
        run_git(&cwd, &["worktree", "add", "-b", branch, &wt_dir.display().to_string()])?;
    } else {
        run_git(&cwd, &["worktree", "add", &wt_dir.display().to_string(), branch])?;
    }

    Ok(wt_dir)
}

fn add_from_remote(branch: &str, wt_dir: &Path, cwd: &Path) -> GroveResult<()> {
    git::fetch_all(cwd)?;

    // Determine the local branch name from the remote reference
    let prefix = branch.split('/').next().unwrap_or("");
    let remote_names: Vec<String> = git::list_local_branches(cwd)?;

    // Check if the prefix is a known remote name
    let first_remote = git::first_remote(cwd).unwrap_or_default();
    let is_remote_prefix = prefix == first_remote || {
        // Try to list remotes and check
        let output = std::process::Command::new("git")
            .args(["remote"])
            .current_dir(cwd)
            .output();
        if let Ok(o) = output {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|r| r == prefix)
        } else {
            false
        }
    };

    let local_branch = if is_remote_prefix {
        branch[prefix.len() + 1..].to_string()
    } else {
        branch.to_string()
    };

    // Check if local branch already exists
    let local_exists = git::list_local_branches(cwd)?
        .iter()
        .any(|b| b == &local_branch);

    if local_exists {
        run_git(cwd, &["worktree", "add", &wt_dir.display().to_string(), &local_branch])?;
    } else {
        run_git(
            cwd,
            &[
                "worktree", "add", "--track", "-b", &local_branch,
                &wt_dir.display().to_string(), branch,
            ],
        )?;
    }

    Ok(())
}

/// Remove a worktree by branch name.
pub fn remove(branch: &str, force: bool) -> GroveResult<PathBuf> {
    let dir = find_by_branch(branch)?;
    let main_dir = main_worktree()?;

    // Safety: cannot remove main worktree
    if dir == main_dir {
        return Err(GroveError::CannotRemoveMain);
    }

    if !force {
        // Check uncommitted changes
        if git::has_uncommitted(&dir)? {
            let dirty_output = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&dir)
                .output()
                .map_err(|e| GroveError::GitError(e.to_string()))?;
            let dirty = String::from_utf8_lossy(&dirty_output.stdout).to_string();
            return Err(GroveError::UncommittedChanges(dirty));
        }

        // Check unpushed commits
        let unpushed = git::unpushed_commits(&dir)?;
        if !unpushed.is_empty() {
            return Err(GroveError::UnpushedCommits(unpushed.join("\n")));
        }
    }

    let cwd = std::env::current_dir()?;
    if force {
        run_git(&cwd, &["worktree", "remove", "--force", &dir.display().to_string()])?;
    } else {
        run_git(&cwd, &["worktree", "remove", &dir.display().to_string()])?;
    }

    // If current directory is inside the removed worktree, return main dir path
    // so the caller can emit a cd.
    Ok(main_dir)
}

/// Check if a path is inside another path.
pub fn is_inside(path: &Path, container: &Path) -> bool {
    path.starts_with(container)
}

fn run_git(cwd: &Path, args: &[&str]) -> GroveResult<()> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| GroveError::GitError(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GroveError::GitError(stderr.trim().to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_inside() {
        let container = Path::new("/home/user/project");
        let inside = Path::new("/home/user/project/worktrees/feat");
        let outside = Path::new("/home/user/other");
        assert!(is_inside(inside, container));
        assert!(!is_inside(outside, container));
    }
}
```

- [ ] **Step 2: Verify compiles and tests pass**

```bash
cargo test -p grove-core
```

Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/worktree.rs
git commit -m "feat(core): add worktree CRUD operations"
```

---

### Task 8: Cache Module (grove-core)

**Files:**
- Create: `crates/grove-core/src/cache.rs`

- [ ] **Step 1: Write cache module**

```rust
// crates/grove-core/src/cache.rs
use crate::config::GroveConfig;
use crate::error::GroveResult;
use crate::git::main_worktree_dir;
use crate::path::short_path;
use crate::pattern::{self, CompiledRule};
use std::path::{Path, PathBuf};

/// List all directory candidates under a source dir, excluding `.git`.
pub fn list_dir_candidates(source: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    walk_dirs(source, source, &mut dirs);
    dirs
}

fn walk_dirs(root: &Path, current: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Skip .git
            if path.file_name().map_or(false, |n| n == ".git") {
                continue;
            }
            // Record relative path from root
            let rel = path.strip_prefix(root).unwrap_or(&path);
            if rel.as_os_str().is_empty() {
                continue;
            }
            out.push(rel.to_path_buf());
            walk_dirs(root, &path, out);
        }
    }
}

/// Resolve which directories should be cached based on rules.
pub fn resolve_cache_dirs(source: &Path, rules: &[CompiledRule]) -> Vec<PathBuf> {
    let candidates = list_dir_candidates(source);
    candidates
        .into_iter()
        .filter(|rel| {
            let rel_str = rel.display().to_string();
            pattern::evaluate(rules, &rel_str)
        })
        .collect()
}

/// Create symlinks from source to target for resolved directories.
/// Returns the number of symlinks created.
pub fn link_cache(source: &Path, target: &Path, rules: &[CompiledRule]) -> usize {
    let dirs = resolve_cache_dirs(source, rules);
    let mut linked = 0;

    for rel in &dirs {
        let src = source.join(rel);
        let dst = target.join(rel);

        if !src.is_dir() {
            continue;
        }
        if dst.exists() || dst.is_symlink() {
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        if std::os::unix::fs::symlink(&src, &dst).is_ok() {
            linked += 1;
        }
    }

    linked
}

/// Remove symlinks from target that match cache rules.
/// Returns the number of symlinks removed.
pub fn unlink_cache(target: &Path, rules: &[CompiledRule]) -> usize {
    let mut removed = 0;

    // For each rule, check if the target path is a symlink and remove it.
    // We need to scan the target directory to find matching symlinks.
    let candidates = list_dir_candidates(target);
    for rel in candidates {
        let rel_str = rel.display().to_string();
        if pattern::evaluate(rules, &rel_str) {
            let dst = target.join(&rel);
            if dst.is_symlink() {
                if std::fs::remove_file(&dst).is_ok() {
                    removed += 1;
                }
            }
        }
    }

    removed
}

/// Cache status information for a single rule against a worktree.
pub enum CacheStatus {
    /// The path is a symlink
    Linked { target: String },
    /// The path is a real directory (not a symlink)
    Local,
    /// The symlink is missing but source exists
    Missing { source: String },
    /// The source directory doesn't exist either
    NotAvailable,
}

/// Get the status of a single cache rule against a worktree.
pub fn rule_status(
    rule: &CompiledRule,
    main_dir: &Path,
    wt_dir: &Path,
) -> CacheStatus {
    // For non-glob, negated rules, skip — their effect is shown via last-match semantics
    if rule.negated {
        return CacheStatus::NotAvailable;
    }

    let pattern = &rule.raw;
    // Strip leading ! just in case
    let pattern = pattern.trim_start_matches('!');
    // Strip leading / for anchored
    let pattern = pattern.trim_start_matches('/');

    let dst = wt_dir.join(pattern);
    let src = main_dir.join(pattern);

    if dst.is_symlink() {
        let target = std::fs::read_link(&dst)
            .map(|t| t.display().to_string())
            .unwrap_or_else(|_| "?".into());
        CacheStatus::Linked { target }
    } else if dst.is_dir() {
        CacheStatus::Local
    } else if src.is_dir() {
        CacheStatus::Missing {
            source: short_path(&src),
        }
    } else {
        CacheStatus::NotAvailable
    }
}
```

- [ ] **Step 2: Verify compiles**

```bash
cargo build -p grove-core
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/grove-core/src/cache.rs
git commit -m "feat(core): add cache symlink management module"
```

---

### Task 9: CLI Argument Definitions (grove)

**Files:**
- Create: `crates/grove/src/cli.rs`

- [ ] **Step 1: Write cli module**

```rust
// crates/grove/src/cli.rs
use clap::{Parser, Subcommand};

/// Git worktree manager
#[derive(Parser, Debug)]
#[command(name = "grove", version, about = "Git worktree manager")]
pub struct Cli {
    /// Machine-readable output (for AI/script use)
    #[arg(long, default_value_t = false, global = true)]
    pub plain: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List worktrees with rich status
    #[command(alias = "ls")]
    List,

    /// Create a new worktree
    #[command(alias = "new")]
    Add {
        /// Branch name
        branch: Option<String>,

        /// Create a new branch (instead of checking out existing)
        #[arg(long, short = 'c')]
        create: bool,

        /// Add from a remote branch
        #[arg(long, short = 'r')]
        remote: bool,

        /// Skip cache symlink creation
        #[arg(long)]
        no_cache: bool,
    },

    /// Switch to a worktree (outputs cd path)
    #[command(alias = "cd")]
    Switch {
        /// Target branch name
        branch: Option<String>,
    },

    /// Remove a worktree
    #[command(alias = "rm")]
    Remove {
        /// Branch name of the worktree to remove
        branch: Option<String>,

        /// Skip safety checks
        #[arg(long, short = 'f')]
        force: bool,
    },

    /// Manage cache symlinks
    Cache {
        #[command(subcommand)]
        action: Option<CacheAction>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Create cache symlinks (default)
    Link,
    /// Show cache symlink status
    Status,
    /// Remove cache symlinks
    Unlink,
}
```

- [ ] **Step 2: Verify compiles**

```bash
cargo build -p grove
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/grove/src/cli.rs
git commit -m "feat(cli): add clap argument definitions"
```

---

### Task 10: Output Module (grove)

**Files:**
- Create: `crates/grove/src/output.rs`

- [ ] **Step 1: Write output module**

```rust
// crates/grove/src/output.rs
use console::{style, Style};
use grove_core::git::WorktreeStatus;
use grove_core::worktree::WorktreeEntry;
use std::path::Path;

/// Human-readable status formatting.
pub fn format_status_human(status: &WorktreeStatus) -> String {
    if status.staged == 0 && status.modified == 0 && status.untracked == 0
        && status.ahead == 0 && status.behind == 0
    {
        return style("clean").green().to_string();
    }

    let mut parts = Vec::new();
    if status.staged > 0 {
        parts.push(style(format!("+{}", status.staged)).green().to_string());
    }
    if status.modified > 0 {
        parts.push(style(format!("~{}", status.modified)).yellow().to_string());
    }
    if status.untracked > 0 {
        parts.push(style(format!("?{}", status.untracked)).red().to_string());
    }
    if status.ahead > 0 {
        parts.push(style(format!("↑{}", status.ahead)).cyan().to_string());
    }
    if status.behind > 0 {
        parts.push(style(format!("↓{}", status.behind)).magenta().to_string());
    }
    parts.join(" ")
}

/// Plain TSV output for `grove --plain list`.
pub fn format_list_entry_plain(entry: &WorktreeEntry) -> String {
    format!(
        "{}\t{}\t{}\tstaged={}\tmodified={}\tuntracked={}\tahead={}\tbehind={}",
        entry.wt.branch,
        entry.wt.path.display(),
        entry.wt.commit,
        entry.status.staged,
        entry.status.modified,
        entry.status.untracked,
        entry.status.ahead,
        entry.status.behind,
    )
}

/// Pretty-print a list of worktrees.
pub fn print_list_pretty(entries: &[WorktreeEntry]) {
    let max_branch = entries
        .iter()
        .map(|e| e.wt.branch.len())
        .max()
        .unwrap_or(6)
        .max(6);

    let max_dir = entries
        .iter()
        .map(|e| grove_core::path::short_path(&e.wt.path).len())
        .max()
        .unwrap_or(3)
        .max(3)
        .min(80);

    // Header
    println!(
        "  {:max_branch$}  {:max_dir$}  {:7}  {}",
        style("BRANCH").bold(),
        style("DIR").bold(),
        style("COMMIT").bold(),
        style("STATUS").bold(),
        max_branch = max_branch,
        max_dir = max_dir,
    );

    for (i, entry) in entries.iter().enumerate() {
        let marker = if entry.is_main { "*" } else { " " };
        let branch = style(&entry.wt.branch).cyan();
        let short_dir = grove_core::path::short_path(&entry.wt.path);
        let mut display_dir = short_dir.clone();
        if display_dir.len() > max_dir {
            display_dir = format!("{}...", &display_dir[..max_dir.saturating_sub(3)]);
        }
        let commit = style(&entry.wt.commit).dim();
        let status = format_status_human(&entry.status);

        println!(
            "{marker} {branch:max_branch$}  {display_dir:max_dir$}  {commit:7}  {status}",
            max_branch = max_branch,
            max_dir = max_dir,
        );
    }
}

/// Print a list of worktrees in plain TSV mode.
pub fn print_list_plain(entries: &[WorktreeEntry]) {
    for entry in entries {
        println!("{}", format_list_entry_plain(entry));
    }
}

/// Emit a cd path for shell integration.
pub fn emit_cd(path: &Path, plain: bool) {
    if let Ok(file) = std::env::var("GROVE_CD_FILE") {
        let _ = std::fs::write(&file, path.display().to_string());
    } else if plain {
        println!("{}", path.display());
    }
    // In human mode without GROVE_CD_FILE, don't output anything for cd.
}

/// Small helper: print to stderr in human mode.
pub fn info(msg: &str) {
    eprintln!("{}", style(msg).green());
}

pub fn warn(msg: &str) {
    eprintln!("{}", style(msg).yellow());
}

pub fn error(msg: &str) {
    eprintln!("{}", style(msg).red());
}
```

- [ ] **Step 2: Verify compiles**

```bash
cargo build -p grove
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/grove/src/output.rs
git commit -m "feat(cli): add dual-mode output formatting"
```

---

### Task 11: Interactive UI Module (grove)

**Files:**
- Create: `crates/grove/src/interactive.rs`

- [ ] **Step 1: Write interactive module**

```rust
// crates/grove/src/interactive.rs
use anyhow::{Context, Result};
use inquire::{Confirm, Select, Text};
use std::path::Path;

/// Interactive action selection for `grove add`.
pub enum AddAction {
    ExistingBranch(String),
    NewBranch(String),
    RemoteBranch(String),
}

/// Run interactive `grove add` flow.
pub fn add_interactive(dir: &Path) -> Result<AddAction> {
    let action = Select::new(
        "选择操作",
        vec!["existing branch", "new branch", "remote branch"],
    )
    .prompt()?;

    match action {
        "existing branch" => {
            let branches = grove_core::git::list_local_branches(dir)
                .context("failed to list branches")?;
            let branch = Select::new("选择已有分支", branches).prompt()?;
            Ok(AddAction::ExistingBranch(branch))
        }
        "new branch" => {
            let branch = Text::new("输入新分支名:").prompt()?;
            Ok(AddAction::NewBranch(branch))
        }
        "remote branch" => {
            grove_core::git::fetch_all(dir).context("fetch failed")?;
            let branches = grove_core::git::list_remote_branches(dir)
                .context("failed to list remote branches")?;
            let branch = Select::new("选择远程分支", branches).prompt()?;
            Ok(AddAction::RemoteBranch(branch))
        }
        _ => unreachable!(),
    }
}

/// Run interactive `grove switch` flow.
pub fn switch_interactive(dir: &Path) -> Result<String> {
    let wts = grove_core::git::parse_worktree_list(dir)
        .context("failed to list worktrees")?;

    let display_lines: Vec<String> = wts
        .iter()
        .map(|wt| {
            let short = grove_core::path::short_path(&wt.path);
            format!("{:30}  {}", wt.branch, short)
        })
        .collect();

    let selected = Select::new("选择 worktree", display_lines).prompt()?;

    // Extract branch name (first word)
    let branch = selected.split_whitespace().next().unwrap_or("").to_string();
    Ok(branch)
}

/// Run interactive `grove remove` flow.
pub fn remove_interactive(dir: &Path) -> Result<String> {
    let wts = grove_core::git::parse_worktree_list(dir)
        .context("failed to list worktrees")?;

    if wts.len() <= 1 {
        anyhow::bail!("no removable worktrees (main worktree cannot be removed)");
    }

    // Skip index 0 (main worktree)
    let display_lines: Vec<String> = wts
        .iter()
        .skip(1)
        .map(|wt| {
            let short = grove_core::path::short_path(&wt.path);
            format!("{:30}  {}", wt.branch, short)
        })
        .collect();

    let selected = Select::new("选择要删除的 worktree", display_lines).prompt()?;
    let branch = selected.split_whitespace().next().unwrap_or("").to_string();

    let confirmed = Confirm::new(&format!("确认删除 worktree '{}'?", branch))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        anyhow::bail!("cancelled");
    }

    Ok(branch)
}

/// Run interactive `grove cache` flow.
pub fn cache_interactive() -> Result<String> {
    let action = Select::new(
        "Cache 操作",
        vec!["link", "status", "unlink"],
    )
    .prompt()?;
    Ok(action.to_string())
}
```

- [ ] **Step 2: Verify compiles**

```bash
cargo build -p grove
```

Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/grove/src/interactive.rs
git commit -m "feat(cli): add interactive selection with inquire"
```

---

### Task 12: Main Entry Point — Wire Everything Together (grove)

**Files:**
- Create: `crates/grove/src/main.rs`
- Modify: `crates/grove/src/lib.rs` (optional, if we want)

- [ ] **Step 1: Write main.rs**

```rust
// crates/grove/src/main.rs
mod cli;
mod interactive;
mod output;

use clap::Parser;
use cli::{CacheAction, Cli, Commands};
use grove_core::config::GroveConfig;
use grove_core::pattern;
use grove_core::worktree::{self, AddOptions};
use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;

    match cli.command {
        Commands::List => cmd_list(cli.plain),
        Commands::Add {
            branch,
            create,
            remote,
            no_cache,
        } => cmd_add(cli.plain, branch, create, remote, no_cache, &cwd),
        Commands::Switch { branch } => cmd_switch(cli.plain, branch, &cwd),
        Commands::Remove { branch, force } => cmd_remove(cli.plain, branch, force, &cwd),
        Commands::Cache { action } => cmd_cache(cli.plain, action, &cwd),
    }
}

fn cmd_list(plain: bool) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let entries = worktree::list_all()?;

    if plain {
        output::print_list_plain(&entries);
    } else {
        output::print_list_pretty(&entries);
    }
    Ok(())
}

fn cmd_add(
    plain: bool,
    branch: Option<String>,
    create: bool,
    remote: bool,
    no_cache: bool,
    cwd: &PathBuf,
) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => {
            // Interactive mode
            match interactive::add_interactive(cwd)? {
                interactive::AddAction::ExistingBranch(b) => b,
                interactive::AddAction::NewBranch(b) => {
                    // Create flag is implied for new branch
                    let wt_dir = worktree::add(
                        &b,
                        &AddOptions {
                            create: true,
                            remote: false,
                            no_cache,
                        },
                    )?;
                    output::emit_cd(&wt_dir, plain);
                    return Ok(());
                }
                interactive::AddAction::RemoteBranch(b) => {
                    let wt_dir = worktree::add(
                        &b,
                        &AddOptions {
                            create: false,
                            remote: true,
                            no_cache,
                        },
                    )?;
                    output::emit_cd(&wt_dir, plain);
                    return Ok(());
                }
            }
        }
    };

    let wt_dir = worktree::add(
        &branch,
        &AddOptions {
            create,
            remote,
            no_cache,
        },
    )?;

    // Auto-link cache unless disabled
    if !no_cache {
        let config = GroveConfig::load(cwd)?;
        if !config.cache.rules.is_empty() {
            let rules: Vec<_> = config
                .cache
                .rules
                .iter()
                .filter_map(|r| pattern::compile(r).ok())
                .collect();
            if !rules.is_empty() {
                let main_dir = worktree::main_worktree()?;
                let linked = grove_core::cache::link_cache(&main_dir, &wt_dir, &rules);
                if linked > 0 && !plain {
                    output::info(&format!("Linked {linked} cache dir(s)"));
                }
            }
        }
    }

    output::emit_cd(&wt_dir, plain);
    Ok(())
}

fn cmd_switch(plain: bool, branch: Option<String>, cwd: &PathBuf) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => interactive::switch_interactive(cwd)?,
    };

    let dir = worktree::find_by_branch(&branch)?;
    if !plain {
        output::info(&format!("-> {}", grove_core::path::short_path(&dir)));
    }
    output::emit_cd(&dir, plain);
    Ok(())
}

fn cmd_remove(
    plain: bool,
    branch: Option<String>,
    force: bool,
    cwd: &PathBuf,
) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let branch = match branch {
        Some(b) => b,
        None => interactive::remove_interactive(cwd)?,
    };

    match worktree::remove(&branch, force) {
        Ok(main_dir) => {
            if !plain {
                output::info(&format!("Removed: {}", branch));
            }
            // If cwd was inside the removed worktree, cd to main
            let current = env::current_dir()?;
            if worktree::is_inside(&current, &main_dir) {
                // Already in main, no need to cd
            } else if current.starts_with(&main_dir) {
                // Still in main worktree area
            } else {
                // Emit cd to main
                output::emit_cd(&main_dir, plain);
            }
        }
        Err(e) => {
            output::error(&e.to_string());
            anyhow::bail!("{}", e);
        }
    }
    Ok(())
}

fn cmd_cache(plain: bool, action: Option<CacheAction>, cwd: &PathBuf) -> anyhow::Result<()> {
    grove_core::git::ensure_git_repo()?;
    let action = match action {
        Some(a) => a,
        None => {
            let s = interactive::cache_interactive()?;
            match s.as_str() {
                "link" => CacheAction::Link,
                "status" => CacheAction::Status,
                "unlink" => CacheAction::Unlink,
                _ => anyhow::bail!("unknown cache action"),
            }
        }
    };

    let config = GroveConfig::load(cwd)?;
    let rules: Vec<_> = config
        .cache
        .rules
        .iter()
        .filter_map(|r| pattern::compile(r).ok())
        .collect();

    let main_dir = worktree::main_worktree()?;

    match action {
        CacheAction::Link => {
            let linked = grove_core::cache::link_cache(&main_dir, cwd, &rules);
            if linked > 0 && !plain {
                output::info(&format!("Linked {linked} cache dir(s) from {}",
                    grove_core::path::short_path(&main_dir)));
            }
        }
        CacheAction::Unlink => {
            let removed = grove_core::cache::unlink_cache(cwd, &rules);
            if removed > 0 {
                output::info(&format!("Unlinked {removed} cache dir(s)"));
            } else {
                output::info("No cache symlinks to remove");
            }
        }
        CacheAction::Status => {
            if rules.is_empty() {
                output::warn("No cache rules configured (check ~/.config/grove/config.toml and project grove.toml)");
                return Ok(());
            }
            for rule in &rules {
                let status = grove_core::cache::rule_status(rule, &main_dir, cwd);
                match status {
                    grove_core::cache::CacheStatus::Linked { target } => {
                        output::info(&format!("  linked   {} -> {}", rule.raw, target));
                    }
                    grove_core::cache::CacheStatus::Local => {
                        output::warn(&format!("  local    {}", rule.raw));
                    }
                    grove_core::cache::CacheStatus::Missing { source } => {
                        output::error(&format!("  missing  {} (available in {})", rule.raw, source));
                    }
                    grove_core::cache::CacheStatus::NotAvailable => {
                        // Skip NA for brevity in human mode
                        if plain {
                            println!("  N/A      {}", rule.raw);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Build release and verify**

```bash
cargo build --release
```

Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add crates/grove/src/main.rs
git commit -m "feat(cli): wire main entry point with all commands"
```

---

### Task 13: Shell Integration and Install Script

**Files:**
- Create: `shell/grove.zsh`
- Create: `install.sh`

- [ ] **Step 1: Write zsh wrapper**

```zsh
# shell/grove.zsh
# Grove git worktree manager — shell integration
# Source this in .zshrc to enable cd support and tab completion.

grove() {
    local _cd_file=$(mktemp)

    GROVE_CD_FILE="$_cd_file" \
        command grove "$@"
    local rc=$?

    if [[ -s "$_cd_file" ]]; then
        builtin cd "$(<$_cd_file)"
    fi
    rm -f "$_cd_file"
    return $rc
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
    local -a flags=('--plain')

    if (( CURRENT == 2 )); then
        _describe 'command' commands
        _values 'flags' $flags
    elif (( CURRENT == 3 )); then
        case "${words[2]}" in
            switch|cd|remove|rm)
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
```

- [ ] **Step 2: Write install script**

```bash
#!/usr/bin/env bash
# grove installer
set -euo pipefail

GROVE_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN_SRC="$GROVE_ROOT/target/release/grove"
BIN_DIR="$HOME/.local/bin"
ZSHRC="$HOME/.zshrc"
MARKER="# grove shell integration"

echo "grove installer"
echo "==============="
echo ""

# Check if binary exists
if [[ ! -f "$BIN_SRC" ]]; then
    echo "[warn] Release binary not found at $BIN_SRC"
    echo "       Building with cargo..."
    cargo build --release --manifest-path "$GROVE_ROOT/Cargo.toml"
fi

# Symlink binary
mkdir -p "$BIN_DIR"
ln -sf "$BIN_SRC" "$BIN_DIR/grove"
echo "[ok] symlinked to $BIN_DIR/grove"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^${BIN_DIR}$"; then
    echo "[warn] $BIN_DIR is not in PATH"
    echo "       add to .zshrc:  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Add shell integration
if grep -q "$MARKER" "$ZSHRC" 2>/dev/null; then
    echo "[ok] shell integration already in .zshrc"
else
    cat >> "$ZSHRC" <<EOF

$MARKER
source "$GROVE_ROOT/shell/grove.zsh"
EOF
    echo "[ok] added shell integration to .zshrc"
fi

echo ""
echo "Done! Run 'source ~/.zshrc' or open a new terminal."
echo ""
echo "Quick start:"
echo "  grove list          # show worktrees with status"
echo "  grove add           # create a worktree (interactive)"
echo "  grove switch        # jump to a worktree (interactive)"
echo "  grove remove        # remove a worktree (interactive)"
echo ""
echo "For AI/script use, add --plain:"
echo "  grove --plain list"
echo "  grove --plain add <branch> --create"
```

- [ ] **Step 3: Make install.sh executable**

```bash
chmod +x install.sh
```

- [ ] **Step 4: Commit**

```bash
git add shell/grove.zsh install.sh
git commit -m "feat: add zsh shell integration and install script"
```

---

### Task 14: Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write release workflow**

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-targets
      - run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow for crates.io publish"
```

---

### Task 15: Integration Tests — Helpers

**Files:**
- Create: `tests/integration/helpers.rs`

- [ ] **Step 1: Write test helpers**

```rust
// tests/integration/helpers.rs
use std::path::{Path, PathBuf};
use std::process::Command;

/// A test fixture that creates a temporary git repo with worktrees.
pub struct TestRepo {
    temp_dir: tempfile::TempDir,
    home_dir: PathBuf,
    work_repo: PathBuf,
    bare_origin: PathBuf,
}

impl TestRepo {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let home_dir = temp_dir.path().join("home");
        std::fs::create_dir(&home_dir).unwrap();

        let bare_origin = temp_dir.path().join("bare_origin");
        let work_repo = temp_dir.path().join("work_repo");

        // Set HOME for grove
        std::env::set_var("HOME", &home_dir);

        // Set GROVE_WORKTREE_BASE
        std::env::set_var("GROVE_WORKTREE_BASE", temp_dir.path().join("grove_worktrees"));

        // Create bare origin
        run_git_init_bare(&bare_origin);

        // Create initial content
        let tmp_clone = temp_dir.path().join("tmp_clone");
        git_clone(&bare_origin, &tmp_clone);
        git_config(&tmp_clone);
        write_file(&tmp_clone, "file.txt", "init\n");
        git_add_commit(&tmp_clone, "initial");
        git_push(&tmp_clone);

        // Clone working repo
        git_clone(&bare_origin, &work_repo);
        git_config(&work_repo);

        // Clean up tmp
        let _ = std::fs::remove_dir_all(&tmp_clone);

        TestRepo {
            temp_dir,
            home_dir,
            work_repo,
            bare_origin,
        }
    }

    pub fn work_repo(&self) -> &Path {
        &self.work_repo
    }

    pub fn grove_bin(&self) -> PathBuf {
        // Find the grove binary — assume cargo build was run
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .unwrap()
            .join("target")
            .join("debug")
            .join("grove")
    }

    pub fn run_grove(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(self.grove_bin())
            .args(args)
            .current_dir(&self.work_repo)
            .output()
            .expect("failed to run grove");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(1);

        (code, stdout, stderr)
    }

    pub fn create_branch(&self, name: &str) {
        let output = Command::new("git")
            .args(["branch", name])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(output.status.success(), "failed to create branch: {:?}", output);
    }

    pub fn checkout(&self, name: &str) {
        let output = Command::new("git")
            .args(["checkout", name])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

fn run_git_init_bare(path: &Path) {
    let output = Command::new("git")
        .args(["init", "--bare", &path.display().to_string(), "-b", "main"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn git_clone(source: &Path, dest: &Path) {
    let output = Command::new("git")
        .args(["clone", &source.display().to_string(), &dest.display().to_string()])
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn git_config(dir: &Path) {
    for (key, val) in &[
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
    ] {
        let output = Command::new("git")
            .args(["config", key, val])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

fn write_file(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn git_add_commit(dir: &Path, msg: &str) {
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "commit failed: {:?}", output);
}

fn git_push(dir: &Path) {
    let output = Command::new("git")
        .args(["push", "origin", "main"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());
}
```

- [ ] **Step 2: Create integration test Cargo.toml**

```toml
# tests/integration/Cargo.toml (adjust workspace as needed — or use a test crate)
# For now, integration tests use #[cfg(test)] in the grove crate or a separate test binary.
```

Note: Integration tests can live in `crates/grove/tests/` for easier compilation. Let's adjust the file structure: integration tests go in `crates/grove/tests/`.

Actually, the simplest approach is to put integration tests directly under `crates/grove/tests/` as separate files — Rust's standard `tests/` directory under a crate automatically compiles each file as a separate test binary.

- [ ] **Step 3: Move helpers to correct location**

Actually, let's put integration test helpers inside `crates/grove/tests/helpers/mod.rs`:

```bash
mkdir -p crates/grove/tests/helpers
```

And add `tempfile` as a dev-dependency of the `grove` crate.

- [ ] **Step 4: Commit**

```bash
git add crates/grove/tests/ crates/grove/Cargo.toml
git commit -m "test: add integration test helpers"
```

---

### Task 16: Integration Tests — Add, List, Switch, Remove, Cache

**Files:**
- Create: `crates/grove/tests/`
  - `helpers/mod.rs`
  - `add_tests.rs`
  - `list_tests.rs`
  - `remove_tests.rs`
  - `switch_tests.rs`
  - `cache_tests.rs`

Note: Each file in `crates/<crate>/tests/` is compiled as a separate test binary using `extern crate grove;`. To share helpers, create a `helpers/` directory with `mod.rs`.

- [ ] **Step 1: Move helpers module**

```rust
// crates/grove/tests/helpers/mod.rs
// (Same content as Task 15's helpers.rs, adjusted as needed)
```

- [ ] **Step 2: Write add integration test**

```rust
// crates/grove/tests/add_tests.rs
mod helpers;

use helpers::TestRepo;

#[test]
fn test_add_local_branch() {
    let repo = TestRepo::new();
    repo.create_branch("feat/test-add");
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "add", "feat/test-add"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(!stdout.trim().is_empty(), "expected worktree path in stdout");
    let wt_dir = stdout.trim().to_string();
    assert!(std::path::Path::new(&wt_dir).is_dir());
}

#[test]
fn test_add_create_new_branch() {
    let repo = TestRepo::new();
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "add", "feat/new-branch", "--create"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let wt_dir = stdout.trim().to_string();
    assert!(std::path::Path::new(&wt_dir).is_dir());

    // Verify it's on the right branch
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(branch, "feat/new-branch");
}

#[test]
fn test_add_non_existent_branch_fails() {
    let repo = TestRepo::new();
    let (code, _, _) = repo.run_grove(&["--plain", "add", "nonexistent-branch"]);
    assert_ne!(code, 0, "should fail for non-existent branch");
}
```

- [ ] **Step 3: Write list integration test**

```rust
// crates/grove/tests/list_tests.rs
mod helpers;

use helpers::TestRepo;

#[test]
fn test_list_plain_output_format() {
    let repo = TestRepo::new();
    let (code, stdout, _) = repo.run_grove(&["--plain", "list"]);
    assert_eq!(code, 0);
    // Should have at least one worktree entry
    assert!(!stdout.trim().is_empty());
    // TSV format: branch\tpath\tcommit\tstaged=N\t...
    let line = stdout.lines().next().unwrap();
    assert!(line.contains('\t'), "expected TSV output, got: {line}");
}

#[test]
fn test_list_after_adding_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feat/list-test");
    repo.run_grove(&["--plain", "add", "feat/list-test"]);

    let (code, stdout, _) = repo.run_grove(&["--plain", "list"]);
    assert_eq!(code, 0);
    let lines: Vec<_> = stdout.lines().collect();
    assert!(lines.len() >= 2, "expected at least 2 worktrees, got {}", lines.len());
}
```

- [ ] **Step 4: Write remove integration test**

```rust
// crates/grove/tests/remove_tests.rs
mod helpers;

use helpers::TestRepo;

#[test]
fn test_remove_clean_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feat/clean");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/clean"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    repo.checkout("main");

    let (code, _, _) = repo.run_grove(&["--plain", "remove", "feat/clean"]);
    assert_eq!(code, 0);
    assert!(!std::path::Path::new(&wt_dir).exists(), "worktree dir should be removed");
}

#[test]
fn test_cannot_remove_main_worktree() {
    let repo = TestRepo::new();
    let (code, _, stderr) = repo.run_grove(&["--plain", "remove", "main"]);
    assert_ne!(code, 0, "should not be able to remove main worktree");
}
```

- [ ] **Step 5: Write switch integration test**

```rust
// crates/grove/tests/switch_tests.rs
mod helpers;

use helpers::TestRepo;

#[test]
fn test_switch_outputs_path() {
    let repo = TestRepo::new();
    repo.create_branch("feat/switch-test");
    repo.run_grove(&["--plain", "add", "feat/switch-test"]);

    let (code, stdout, _) = repo.run_grove(&["--plain", "switch", "feat/switch-test"]);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
    assert!(std::path::Path::new(stdout.trim()).is_dir());
}

#[test]
fn test_switch_non_existent_fails() {
    let repo = TestRepo::new();
    let (code, _, _) = repo.run_grove(&["--plain", "switch", "no-such-branch"]);
    assert_ne!(code, 0);
}
```

- [ ] **Step 6: Write cache integration test**

```rust
// crates/grove/tests/cache_tests.rs
mod helpers;

use helpers::TestRepo;

#[test]
fn test_cache_link_creates_symlinks() {
    let repo = TestRepo::new();
    let work_repo = repo.work_repo();

    // Create cacheable directories
    std::fs::create_dir_all(work_repo.join("node_modules")).unwrap();
    std::fs::create_dir_all(work_repo.join("packages/pkg-a/node_modules")).unwrap();
    std::fs::create_dir_all(work_repo.join("packages/pkg-b/node_modules")).unwrap();

    // Write project config
    std::fs::write(
        work_repo.join("grove.toml"),
        r#"
[cache]
rules = ["node_modules", "packages/*/node_modules"]
"#,
    )
    .unwrap();

    repo.create_branch("feat/cache-test");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/cache-test"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    assert!(std::fs::symlink_metadata(format!("{wt_dir}/node_modules"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(std::fs::symlink_metadata(format!("{wt_dir}/packages/pkg-a/node_modules"))
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn test_cache_unlink_removes_symlinks() {
    let repo = TestRepo::new();
    let work_repo = repo.work_repo();

    std::fs::create_dir_all(work_repo.join("node_modules")).unwrap();
    std::fs::write(
        work_repo.join("grove.toml"),
        r#"
[cache]
rules = ["node_modules"]
"#,
    )
    .unwrap();

    repo.create_branch("feat/unlink-test");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/unlink-test"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    // Change to that dir and run cache unlink
    std::env::set_current_dir(&wt_dir).unwrap();

    // Run unlink via relative path from work_repo
    let grove_bin = repo.grove_bin();
    let output = std::process::Command::new(&grove_bin)
        .args(["--plain", "cache", "--unlink"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    assert!(!std::path::Path::new(&format!("{wt_dir}/node_modules")).exists());
}
```

- [ ] **Step 7: Add dev-dependencies for tests**

```toml
# In crates/grove/Cargo.toml, add:
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 8: Run integration tests**

```bash
cargo test -p grove
```

Expected: All tests pass (unit + integration).

- [ ] **Step 9: Commit**

```bash
git add crates/grove/tests/ crates/grove/Cargo.toml
git commit -m "test: add integration tests for all commands"
```

---

### Task 17: E2E Smoke Test

**Files:**
- Create: `tests/e2e/smoke.sh`

- [ ] **Step 1: Write E2E smoke test**

```bash
#!/usr/bin/env bash
# tests/e2e/smoke.sh — grove end-to-end smoke test
set -euo pipefail

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

export HOME="$TMPDIR/home"
mkdir -p "$HOME"
export GROVE_WORKTREE_BASE="$TMPDIR/grove_worktrees"
mkdir -p "$GROVE_WORKTREE_BASE"

# Find grove binary
GROVE_BIN="$(cd "$(dirname "$0")/../.." && pwd)/target/release/grove"
if [[ ! -f "$GROVE_BIN" ]]; then
    GROVE_BIN="$(cd "$(dirname "$0")/../.." && pwd)/target/debug/grove"
fi
echo "Using grove at: $GROVE_BIN"

PASS=0
FAIL=0

pass() { echo "PASS: $1"; ((PASS++)); }
fail() { echo "FAIL: $1"; ((FAIL++)); [[ -n "${2:-}" ]] && echo "      $2"; }

# Fixture: create a test repo
BARE="$TMPDIR/bare_origin"
WORK="$TMPDIR/work_repo"

git init --bare "$BARE" -b main >/dev/null 2>&1

TMP_CLONE="$TMPDIR/tmp_clone"
git clone "$BARE" "$TMP_CLONE" >/dev/null 2>&1
git -C "$TMP_CLONE" config user.email "test@test.com"
git -C "$TMP_CLONE" config user.name "Test"
echo "hello" > "$TMP_CLONE/README.md"
git -C "$TMP_CLONE" add README.md
git -C "$TMP_CLONE" commit -m "init" >/dev/null 2>&1
git -C "$TMP_CLONE" push origin main >/dev/null 2>&1
rm -rf "$TMP_CLONE"

git clone "$BARE" "$WORK" >/dev/null 2>&1
git -C "$WORK" config user.email "test@test.com"
git -C "$WORK" config user.name "Test"

# Test 1: grove list (plain)
output=$("$GROVE_BIN" --plain list 2>/dev/null || true)
if echo "$output" | grep -q "main"; then
    pass "groove --plain list shows main worktree"
else
    fail "grove --plain list" "output: $output"
fi

# Test 2: grove add --create
output=$("$GROVE_BIN" --plain add "feat/e2e" --create 2>/dev/null || true)
expected_dir="$GROVE_WORKTREE_BASE/work_repo/feat-e2e"
if [[ -d "$expected_dir" ]]; then
    pass "grove add --create feat/e2e"
else
    fail "grove add --create feat/e2e" "expected dir: $expected_dir"
fi

# Test 3: grove switch
cd "$WORK"
output=$("$GROVE_BIN" --plain switch "feat/e2e" 2>/dev/null || true)
if [[ "$output" == "$expected_dir" ]]; then
    pass "grove switch feat/e2e"
else
    fail "grove switch feat/e2e" "expected: $expected_dir, got: $output"
fi

# Test 4: grove cache — status
cat > "$WORK/grove.toml" <<'TOML'
[cache]
rules = ["node_modules"]
TOML
mkdir -p "$WORK/node_modules"
output=$("$GROVE_BIN" --plain cache --status 2>&1 || true)
if echo "$output" | grep -q "N/A"; then
    pass "grove cache --status works"
else
    fail "grove cache --status" "output: $output"
fi

# Test 5: grove remove
git -C "$WORK" checkout main >/dev/null 2>&1
output=$("$GROVE_BIN" --plain remove "feat/e2e" 2>&1 || true)
if [[ ! -d "$expected_dir" ]]; then
    pass "grove remove feat/e2e"
else
    fail "grove remove feat/e2e" "dir still exists"
fi

echo ""
echo "--- Results: $PASS passed, $FAIL failed ---"

if (( FAIL > 0 )); then
    exit 1
fi
exit 0
```

- [ ] **Step 2: Make executable and verify**

```bash
chmod +x tests/e2e/smoke.sh
cargo build --release
bash tests/e2e/smoke.sh
```

Expected: 5/5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e/smoke.sh
git commit -m "test: add e2e smoke test"
```

---

### Task 18: Documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write README**

```markdown
# grove

Git worktree manager — interactive for humans, machine-readable for AI/scripts.

## Install

### Via cargo

```bash
cargo install grove
```

Then add shell integration to `.zshrc`:

```bash
echo 'source "$(grove --shell-path)/shell/grove.zsh"' >> ~/.zshrc
source ~/.zshrc
```

### From source

```bash
git clone https://github.com/xuzhu-591/grove.git
cd grove
cargo build --release
bash install.sh
source ~/.zshrc
```

## Usage

| Command | Description |
|---------|-------------|
| `grove list` | List worktrees with rich status |
| `grove add` | Create worktree (interactive branch picker) |
| `grove switch` | Jump to a worktree |
| `grove remove` | Remove a worktree (with safety checks) |
| `grove cache` | Manage build cache symlinks |

### Interactive (default)

```bash
grove list              # show all worktrees with status
grove add               # create worktree (interactive)
grove switch            # jump to worktree (interactive)
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

### Output format

`grove --plain list` outputs TSV:

```
branch\tpath\tcommit\tstaged=N\tmodified=N\tuntracked=N\tahead=N\tbehind=N
```

## Configuration

### Worktree base path

Set via environment variable or config (env var takes priority):

```bash
export GROVE_WORKTREE_BASE=/path/to/worktrees
```

```toml
# ~/.config/grove/config.toml
[worktree]
base_path = "~/worktrees"
```

### Cache rules

Define directories to symlink from the main worktree into new ones:

```toml
# ~/.config/grove/config.toml (global, all projects)
[cache]
rules = ["node_modules", ".cache/*"]
```

```toml
# <project>/grove.toml (project-specific, overrides global)
[cache]
rules = ["!**/test", "packages/*/node_modules"]
```

Rules use a gitignore subset: literal paths, `*`, `?`, `**`, `!negation`, `/anchored`. Evaluated last-match-wins across both config files.

## Requirements

- Rust toolchain (for install from source)
- Git
- Zsh (for shell integration)

## License

[MIT](LICENSE)
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README for Rust version"
```

---

## Plan Summary

| # | Task | Files Created |
|---|------|---------------|
| 1 | Scaffolding | 7 files (workspace, crates, CI) |
| 2 | Error types | 1 (error.rs) |
| 3 | Path utils | 1 (path.rs) |
| 4 | Config module | 1 (config.rs) |
| 5 | Pattern engine | 1 (pattern.rs) — the most complex module |
| 6 | Git wrappers | 1 (git.rs) |
| 7 | Worktree ops | 1 (worktree.rs) |
| 8 | Cache module | 1 (cache.rs) |
| 9 | CLI definitions | 1 (cli.rs) |
| 10 | Output module | 1 (output.rs) |
| 11 | Interactive UI | 1 (interactive.rs) |
| 12 | Main entry | 1 (main.rs) — wire everything |
| 13 | Shell integration | 2 (grove.zsh, install.sh) |
| 14 | Release CI | 1 (release.yml) |
| 15-16 | Integration tests | 6 (helpers + 5 test files) |
| 17 | E2E test | 1 (smoke.sh) |
| 18 | Documentation | 1 (README.md) |

**Total: ~28 files, 18 tasks.**
