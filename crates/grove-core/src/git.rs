use crate::error::{GroveError, GroveResult};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git").args(args).current_dir(dir).output()
}

fn git_checked(dir: &Path, args: &[&str]) -> GroveResult<std::process::Output> {
    let output = git(dir, args).map_err(|e| GroveError::GitError(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GroveError::GitError(stderr.trim().to_string()));
    }
    Ok(output)
}

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

pub fn project_name() -> GroveResult<String> {
    let output = git_checked(
        &std::env::current_dir().unwrap(),
        &["remote", "get-url", "origin"],
    )?;
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err(GroveError::NoOriginRemote);
    }
    let name = url.trim_end_matches('/').trim_end_matches(".git");
    let name = name.rsplit('/').next().unwrap_or(name);
    Ok(name.to_string())
}

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

#[derive(Debug, Clone)]
pub struct Worktree {
    pub branch: String,
    pub path: PathBuf,
    pub commit: String,
}

pub fn parse_worktree_list(dir: &Path) -> GroveResult<Vec<Worktree>> {
    let output = git_checked(dir, &["worktree", "list", "--porcelain"])?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch = String::new();
    let mut current_commit = String::new();

    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
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

#[derive(Debug, Clone, Default)]
pub struct WorktreeStatus {
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
    pub ahead: u32,
    pub behind: u32,
}

pub fn parse_status(dir: &Path) -> GroveResult<WorktreeStatus> {
    let output = git(dir, &["status", "--porcelain=v2", "--branch"]);

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Ok(WorktreeStatus::default()),
    };

    let mut status = WorktreeStatus::default();

    for line in text.lines() {
        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.len() >= 3 && !line.starts_with('#') && !line.starts_with('?') {
            let chars: Vec<char> = line.chars().collect();
            let x = chars[0];
            let y = chars[1];
            if x != '.' && x != '?' {
                status.staged += 1;
            }
            if y != '.' && y != '?' {
                status.modified += 1;
            }
        } else if line.starts_with('?') {
            status.untracked += 1;
        }
    }

    Ok(status)
}

pub fn has_uncommitted(dir: &Path) -> GroveResult<bool> {
    let output = git(dir, &["status", "--porcelain"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(!text.trim().is_empty())
}

pub fn unpushed_commits(dir: &Path) -> GroveResult<Vec<String>> {
    let main_dir = main_worktree_dir()?;
    let output = git(
        dir,
        &["log", "--oneline", &format!("{}..HEAD", main_dir.display())],
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(text.lines().map(|s| s.to_string()).collect())
}

pub fn list_local_branches(dir: &Path) -> GroveResult<Vec<String>> {
    let output = git_checked(dir, &["branch", "--format=%(refname:short)"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn list_remote_branches(dir: &Path) -> GroveResult<Vec<String>> {
    let output = git_checked(dir, &["branch", "-r", "--format=%(refname:short)"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.ends_with("/HEAD"))
        .collect())
}

pub fn fetch_all(dir: &Path) -> GroveResult<()> {
    git_checked(dir, &["fetch", "--all", "--prune"])?;
    Ok(())
}

pub fn first_remote(dir: &Path) -> GroveResult<String> {
    let output = git_checked(dir, &["remote"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .map(|s| s.to_string())
        .ok_or(GroveError::GitError("no remotes configured".into()))
}
