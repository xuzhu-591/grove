use std::path::{Path, PathBuf};

pub fn worktree_path(base: &Path, project_name: &str, branch: &str) -> PathBuf {
    let safe_branch = branch.replace('/', "-");
    base.join(project_name).join(safe_branch)
}

pub fn default_worktree_base() -> PathBuf {
    dirs_home().join(".grove").join("worktrees")
}

pub fn resolve_worktree_base() -> PathBuf {
    std::env::var("GROVE_WORKTREE_BASE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_worktree_base())
}

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
        assert_eq!(
            p,
            Path::new("/home/user/.grove/worktrees/myproject/feat-login")
        );
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
