use crate::error::{GroveError, GroveResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct GroveConfig {
    #[serde(default)]
    pub cache: CacheSection,

    #[serde(default)]
    pub worktree: WorktreeSection,
}

#[derive(Debug, Deserialize, Default)]
pub struct CacheSection {
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct WorktreeSection {
    #[serde(default)]
    pub base_path: Option<String>,
}

impl GroveConfig {
    pub fn load(repo_root: &Path) -> GroveResult<Self> {
        let global_dir = global_config_dir();
        Self::load_inner(repo_root, &global_dir)
    }

    fn load_inner(repo_root: &Path, config_dir: &Path) -> GroveResult<Self> {
        let mut merged = GroveConfig::default();

        // 1. Global config
        let global_path = config_dir.join("config.toml");
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| GroveError::ConfigError(path.to_path_buf(), e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| GroveError::ConfigError(path.to_path_buf(), e.to_string()))
    }

    fn merge(&mut self, other: GroveConfig) {
        self.cache.rules.extend(other.cache.rules);
        if other.worktree.base_path.is_some() {
            self.worktree.base_path = other.worktree.base_path;
        }
    }
}

pub fn resolve_worktree_base(config: &GroveConfig) -> PathBuf {
    if let Ok(env_base) = std::env::var("GROVE_WORKTREE_BASE") {
        return PathBuf::from(env_base);
    }
    if let Some(ref base) = config.worktree.base_path {
        return shellexpand::tilde(base).into_owned().into();
    }
    crate::path::default_worktree_base()
}

fn global_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config").join("grove")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(merged.cache.rules[0], "node_modules");
        assert_eq!(merged.cache.rules[1], "packages/*/node_modules");
    }

    #[test]
    fn test_config_file_not_found_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let config = GroveConfig::load_inner(tmp.path(), tmp.path());
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

        let config = GroveConfig::load_inner(tmp.path(), tmp.path()).unwrap();
        assert_eq!(config.cache.rules.len(), 1);
        assert_eq!(config.cache.rules[0], "node_modules");
    }
}
