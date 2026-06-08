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
            if path.file_name().map_or(false, |n| n == ".git") {
                continue;
            }
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
pub fn unlink_cache(target: &Path, rules: &[CompiledRule]) -> usize {
    let candidates = list_dir_candidates(target);
    let mut removed = 0;

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

/// Cache status for display.
pub enum CacheStatus {
    Linked { target: String },
    Local,
    Missing { source: String },
    NotAvailable,
}

pub fn rule_status(rule: &CompiledRule, main_dir: &Path, wt_dir: &Path) -> CacheStatus {
    if rule.negated {
        return CacheStatus::NotAvailable;
    }

    let pattern = rule.raw.trim_start_matches('!').trim_start_matches('/');

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
