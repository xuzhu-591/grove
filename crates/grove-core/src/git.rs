use crate::error::{GroveError, GroveResult};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git").args(args).current_dir(dir).output()
}

pub(crate) fn git_checked(dir: &Path, args: &[&str]) -> GroveResult<std::process::Output> {
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
    pub prunable: bool,
    pub locked: bool,
    pub bare: bool,
}

pub fn parse_worktree_list(dir: &Path) -> GroveResult<Vec<Worktree>> {
    let output = git_checked(dir, &["worktree", "list", "--porcelain", "-z"])?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    for record in text.split("\0\0").filter(|r| !r.is_empty()) {
        let mut wt = Worktree {
            branch: "(detached)".into(),
            path: PathBuf::new(),
            commit: String::new(),
            prunable: false,
            locked: false,
            bare: false,
        };
        for field in record.split('\0') {
            if let Some(path) = field.strip_prefix("worktree ") {
                wt.path = PathBuf::from(path);
            } else if let Some(head) = field.strip_prefix("HEAD ") {
                wt.commit = head.to_string();
            } else if let Some(branch) = field.strip_prefix("branch refs/heads/") {
                wt.branch = branch.to_string();
            } else if field == "locked" || field.starts_with("locked ") {
                wt.locked = true;
            } else if field == "prunable" || field.starts_with("prunable ") {
                wt.prunable = true;
            } else if field == "bare" {
                wt.bare = true;
            }
        }
        if wt.path.as_os_str().is_empty() {
            return Err(GroveError::GitError("invalid worktree record".into()));
        }
        worktrees.push(wt);
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

impl WorktreeStatus {
    pub fn is_dirty(&self) -> bool {
        self.staged > 0 || self.modified > 0 || self.untracked > 0
    }
}

pub fn parse_status(dir: &Path) -> GroveResult<WorktreeStatus> {
    let output = git_checked(
        dir,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    parse_status_output(&output.stdout)
}

fn parse_status_output(output: &[u8]) -> GroveResult<WorktreeStatus> {
    let mut status = WorktreeStatus::default();
    let mut records = output.split(|b| *b == 0).filter(|r| !r.is_empty());
    while let Some(record) = records.next() {
        if record.starts_with(b"# branch.ab ") {
            let text = String::from_utf8_lossy(&record[12..]);
            let mut parts = text.split_whitespace();
            let mut number = |prefix| -> GroveResult<u32> {
                parts
                    .next()
                    .and_then(|p| p.strip_prefix(prefix))
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| GroveError::GitError("invalid ahead/behind status".into()))
            };
            status.ahead = number('+')?;
            status.behind = number('-')?;
        } else if record.starts_with(b"# ") {
            continue;
        } else if record.starts_with(b"? ") {
            status.untracked += 1;
        } else if matches!(record.first(), Some(b'1' | b'2' | b'u')) {
            // Porcelain v2 starts with a record type, then XY and submodule state.
            if record.len() < 10 || record[1] != b' ' || record[4] != b' ' {
                return Err(GroveError::GitError("invalid file status record".into()));
            }
            if record[2] != b'.' {
                status.staged += 1;
            }
            if record[3] != b'.' || (record[5] == b'S' && record[6..9] != *b"...") {
                status.modified += 1;
            }
            // Rename/copy records carry a second NUL-delimited original path.
            if record[0] == b'2' && records.next().is_none() {
                return Err(GroveError::GitError("missing rename source path".into()));
            }
        } else {
            return Err(GroveError::GitError(
                "unrecognized file status record".into(),
            ));
        }
    }
    Ok(status)
}

pub fn has_uncommitted(dir: &Path) -> GroveResult<bool> {
    Ok(parse_status(dir)?.is_dirty())
}

pub fn resolve_commit(dir: &Path, reference: &str) -> GroveResult<String> {
    let output = git_checked(
        dir,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn upstream(dir: &Path) -> GroveResult<Option<String>> {
    let branch = git_checked(dir, &["symbolic-ref", "--quiet", "HEAD"])?;
    let branch = String::from_utf8_lossy(&branch.stdout);
    let output = git_checked(
        dir,
        &["for-each-ref", "--format=%(upstream)", branch.trim()],
    )?;
    let reference = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!reference.is_empty()).then_some(reference))
}

pub fn is_ancestor(dir: &Path, head: &str, base: &str) -> GroveResult<bool> {
    let output = git(dir, &["merge-base", "--is-ancestor", head, base])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(GroveError::GitError(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )),
    }
}

pub fn operation_in_progress(dir: &Path) -> GroveResult<bool> {
    let output = git_checked(dir, &["rev-parse", "--absolute-git-dir"])?;
    let git_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    for name in [
        "MERGE_HEAD",
        "rebase-merge",
        "rebase-apply",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "sequencer",
        "BISECT_START",
    ] {
        if git_dir.join(name).try_exists()? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn unpushed_commits(dir: &Path) -> GroveResult<Vec<String>> {
    let reference = match upstream(dir)? {
        Some(reference) => reference,
        None => parse_worktree_list(dir)?
            .first()
            .filter(|w| !w.commit.is_empty())
            .ok_or_else(|| GroveError::GitError("no main worktree commit".into()))?
            .commit
            .clone(),
    };
    let base = resolve_commit(dir, &reference)?;
    let output = git_checked(dir, &["log", "--oneline", &format!("{base}..HEAD")])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
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

/// Local branches whose tip is reachable from `base` (i.e. merged into it).
/// Mirrors `git branch --merged <base>`: detects merge-commit / fast-forward
/// merges; squash/rebase merges are not detectable from local git topology.
pub fn merged_branches(dir: &Path, base: &str) -> GroveResult<Vec<String>> {
    let output = git_checked(
        dir,
        &["branch", "--merged", base, "--format=%(refname:short)"],
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn fetch_all(dir: &Path) -> GroveResult<()> {
    git_checked(dir, &["fetch", "--all", "--prune"])?;
    Ok(())
}

/// Fast-forward the current branch to its configured upstream.
///
/// This is equivalent to the merge phase of `git pull --ff-only`, after the
/// caller has refreshed remote refs. It never creates a merge commit.
pub fn merge_upstream_fast_forward(dir: &Path) -> GroveResult<()> {
    git_checked(dir, &["merge", "--ff-only", "@{upstream}"])?;
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
