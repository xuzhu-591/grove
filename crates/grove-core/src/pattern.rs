use crate::error::{GroveError, GroveResult};

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub raw: String,
    pub negated: bool,
    anchored: bool,
    kind: RuleKind,
}

#[derive(Debug, Clone)]
enum RuleKind {
    /// Exact basename match (no wildcards, no `/`): match at any depth
    Basename(String),
    /// Full pattern with `/` but no `**`: match exact path or trailing suffix
    ExactPath(String),
    /// Contains `**/` prefix: match anywhere
    Anywhere(String),
    /// Contains `/**` suffix: match prefix and all descendants
    Prefix(String),
    /// Contains `/**/` middle: match prefix + any path + suffix
    Recursive {
        prefix: String,
        suffix: String,
    },
    /// Pattern contains `*` or `?` WITH `/`: glob match per path segment
    Glob(String),
    /// Pattern contains `*` or `?` without `/`: match basename at any depth
    GlobBasename(String),
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
    if pattern.contains("/**/") {
        let parts: Vec<&str> = pattern.splitn(2, "/**/").collect();
        return RuleKind::Recursive {
            prefix: parts[0].to_string(),
            suffix: parts[1].to_string(),
        };
    }

    if let Some(rest) = pattern.strip_prefix("**/") {
        return RuleKind::Anywhere(rest.to_string());
    }

    if let Some(prefix) = pattern.strip_suffix("/**") {
        return RuleKind::Prefix(prefix.to_string());
    }

    if pattern.contains('*') || pattern.contains('?') {
        if pattern.contains('/') {
            return RuleKind::Glob(pattern.to_string());
        }
        // Glob without / — matches basename at any depth
        return RuleKind::GlobBasename(pattern.to_string());
    }

    if pattern.contains('/') {
        return RuleKind::ExactPath(pattern.to_string());
    }

    RuleKind::Basename(pattern.to_string())
}

/// Test whether a compiled rule matches a repo-relative directory path.
pub fn matches(rule: &CompiledRule, rel_path: &str) -> bool {
    match &rule.kind {
        RuleKind::Basename(name) => {
            if rule.anchored {
                rel_path == name || rel_path.starts_with(&format!("{}/", name))
            } else {
                rel_path == name || rel_path.ends_with(&format!("/{}", name))
            }
        }
        RuleKind::ExactPath(path) => {
            rel_path == path || rel_path.ends_with(&format!("/{}", path))
        }
        RuleKind::Anywhere(suffix) => {
            rel_path == suffix || rel_path.ends_with(&format!("/{}", suffix))
        }
        RuleKind::Prefix(prefix) => {
            rel_path == prefix || rel_path.starts_with(&format!("{}/", prefix))
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
        RuleKind::Glob(pattern) => glob_match_segments(pattern, rel_path),
        RuleKind::GlobBasename(pattern) => {
            // Match last segment against glob, at any depth
            let basename = rel_path.rsplit('/').next().unwrap_or(rel_path);
            if rule.anchored {
                rel_path == basename && glob_segment_match(pattern, basename)
            } else {
                glob_segment_match(pattern, basename)
            }
        }
    }
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
                    return true;
                }
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

/// Evaluate a list of rules against a path. Last-match-wins.
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
        assert!(matches(&rule, "build/output"));
        assert!(!matches(&rule, "packages/build"));
    }

    #[test]
    fn test_exact_path_match() {
        let rule = compile("packages/pkg-a/node_modules").unwrap();
        assert!(matches(&rule, "packages/pkg-a/node_modules"));
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
}
