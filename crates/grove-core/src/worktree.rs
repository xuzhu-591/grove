use crate::error::{GroveError, GroveResult};
use crate::git::{self, Worktree as GitWorktree, WorktreeStatus};
use crate::path;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct AddOptions {
    pub create: bool,
    pub remote: bool,
    pub no_cache: bool,
}

pub struct WorktreeEntry {
    pub wt: GitWorktree,
    pub status: WorktreeStatus,
    pub is_main: bool,
}

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

pub fn main_worktree() -> GroveResult<PathBuf> {
    git::main_worktree_dir()
}

pub fn add(branch: &str, opts: &AddOptions) -> GroveResult<PathBuf> {
    let base = path::resolve_worktree_base();
    let project = git::project_name()?;
    let wt_dir = path::worktree_path(&base, &project, branch);
    let cwd = std::env::current_dir()?;

    if opts.remote {
        add_from_remote(branch, &wt_dir, &cwd)?;
    } else if opts.create {
        run_git(
            &cwd,
            &["worktree", "add", "-b", branch, &wt_dir.display().to_string()],
        )?;
    } else {
        run_git(
            &cwd,
            &["worktree", "add", &wt_dir.display().to_string(), branch],
        )?;
    }

    Ok(wt_dir)
}

fn add_from_remote(branch: &str, wt_dir: &Path, cwd: &Path) -> GroveResult<()> {
    git::fetch_all(cwd)?;

    let prefix = branch.split('/').next().unwrap_or("");

    // Check if the prefix is a known remote name
    let is_remote_prefix = {
        let output = Command::new("git")
            .args(["remote"])
            .current_dir(cwd)
            .output();
        match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|r| r == prefix),
            Err(_) => false,
        }
    };

    let local_branch = if is_remote_prefix {
        branch[prefix.len() + 1..].to_string()
    } else {
        branch.to_string()
    };

    let local_exists = git::list_local_branches(cwd)?
        .iter()
        .any(|b| b == &local_branch);

    if local_exists {
        run_git(
            cwd,
            &[
                "worktree",
                "add",
                &wt_dir.display().to_string(),
                &local_branch,
            ],
        )?;
    } else {
        run_git(
            cwd,
            &[
                "worktree",
                "add",
                "--track",
                "-b",
                &local_branch,
                &wt_dir.display().to_string(),
                branch,
            ],
        )?;
    }

    Ok(())
}

pub fn remove(branch: &str, force: bool) -> GroveResult<PathBuf> {
    let dir = find_by_branch(branch)?;
    let main_dir = main_worktree()?;

    if dir == main_dir {
        return Err(GroveError::CannotRemoveMain);
    }

    if !force {
        if git::has_uncommitted(&dir)? {
            let dirty_output = Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&dir)
                .output()
                .map_err(|e| GroveError::GitError(e.to_string()))?;
            let dirty = String::from_utf8_lossy(&dirty_output.stdout).to_string();
            return Err(GroveError::UncommittedChanges(dirty));
        }

        let unpushed = git::unpushed_commits(&dir)?;
        if !unpushed.is_empty() {
            return Err(GroveError::UnpushedCommits(unpushed.join("\n")));
        }
    }

    let cwd = std::env::current_dir()?;
    if force {
        run_git(
            &cwd,
            &["worktree", "remove", "--force", &dir.display().to_string()],
        )?;
    } else {
        run_git(
            &cwd,
            &["worktree", "remove", &dir.display().to_string()],
        )?;
    }

    Ok(main_dir)
}

pub fn is_inside(path: &Path, container: &Path) -> bool {
    path.starts_with(container)
}

fn run_git(cwd: &Path, args: &[&str]) -> GroveResult<()> {
    let output = Command::new("git")
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
