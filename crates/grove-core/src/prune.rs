use crate::error::{GroveError, GroveResult};
use crate::git::{self, Worktree};
use crate::worktree::{self, MainWorktreeSync};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PruneState {
    Candidate,
    Skipped,
    Removed,
    Failed,
}

impl PruneState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Skipped => "skipped",
            Self::Removed => "removed",
            Self::Failed => "failed",
        }
    }
}

pub struct PruneEntry {
    pub wt: Worktree,
    pub state: PruneState,
    pub reason: String,
}

pub struct PrunePlan {
    pub base: Worktree,
    pub entries: Vec<PruneEntry>,
    cwd: PathBuf,
}

/// Refresh just as list does, but never accept a failed or incomplete refresh.
/// The preview and every removal are bound to one full base commit.
pub fn prepare(cwd: &Path) -> GroveResult<PrunePlan> {
    let wts = git::parse_worktree_list(cwd)?;
    match worktree::sync_main_before_list(&wts, cwd)? {
        MainWorktreeSync::UpToDate | MainWorktreeSync::Updated { .. } => {}
        other => {
            return Err(GroveError::GitError(format!(
                "cannot establish refreshed prune base: {other:?}"
            )))
        }
    }
    let wts = git::parse_worktree_list(cwd)?;
    let base = wts
        .first()
        .filter(|w| !w.bare && w.branch != "(detached)" && !w.commit.is_empty())
        .ok_or_else(|| GroveError::GitError("no main worktree branch for prune".into()))?
        .clone();
    let cwd = cwd.canonicalize()?;
    let mut entries = Vec::new();
    for wt in &wts {
        let (state, reason) = assessment(wt, &base, &cwd, &wts);
        entries.push(PruneEntry {
            wt: wt.clone(),
            state,
            reason,
        });
    }
    Ok(PrunePlan { base, entries, cwd })
}

fn assessment(
    wt: &Worktree,
    base: &Worktree,
    cwd: &Path,
    wts: &[Worktree],
) -> (PruneState, String) {
    match skip_reason(wt, base, cwd, wts) {
        Ok(Some(reason)) => (PruneState::Skipped, reason.into()),
        Ok(None) => (
            PruneState::Candidate,
            "merged and clean; ignored files will be removed".into(),
        ),
        Err(error) => (PruneState::Skipped, format!("unable to verify: {error}")),
    }
}

fn absolute_git_path(dir: &Path, args: &[&str]) -> GroveResult<PathBuf> {
    let output = git::git_checked(dir, args)?;
    Ok(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).canonicalize()?)
}

fn skip_reason(
    wt: &Worktree,
    base: &Worktree,
    cwd: &Path,
    wts: &[Worktree],
) -> GroveResult<Option<&'static str>> {
    if wt.path == base.path {
        return Ok(Some("main worktree"));
    }
    if wt.locked {
        return Ok(Some("locked worktree"));
    }
    if wt.bare || wt.branch == "(detached)" {
        return Ok(Some("no branch"));
    }
    if wt.prunable || !wt.path.try_exists()? {
        return Ok(Some("missing or stale worktree"));
    }
    let path = wt.path.canonicalize()?;
    if cwd.starts_with(&path) {
        return Ok(Some("current worktree"));
    }
    for other in wts.iter().filter(|other| other.path != wt.path) {
        if other
            .path
            .canonicalize()
            .is_ok_and(|p| p.starts_with(&path))
        {
            return Ok(Some("contains another worktree"));
        }
    }
    let top = absolute_git_path(&path, &["rev-parse", "--show-toplevel"])?;
    let common = ["rev-parse", "--path-format=absolute", "--git-common-dir"];
    if top != path || absolute_git_path(&path, &common)? != absolute_git_path(&base.path, &common)?
    {
        return Ok(Some("worktree identity changed"));
    }
    let branch = git::git_checked(&path, &["symbolic-ref", "--quiet", "HEAD"])?;
    if String::from_utf8_lossy(&branch.stdout).trim() != format!("refs/heads/{}", wt.branch)
        || git::resolve_commit(&path, "HEAD")? != wt.commit
    {
        return Ok(Some("worktree HEAD changed"));
    }
    if git::operation_in_progress(&path)? {
        return Ok(Some("Git operation in progress"));
    }
    if !git::is_ancestor(&base.path, &wt.commit, &base.commit)? {
        return Ok(Some("not merged into base"));
    }
    if git::parse_status(&path)?.is_dirty() {
        return Ok(Some("uncommitted or untracked changes"));
    }
    Ok(None)
}

fn base_unchanged(plan: &PrunePlan, wts: &[Worktree]) -> GroveResult<bool> {
    let Some(base) = wts.first() else {
        return Ok(false);
    };
    Ok(base.path == plan.base.path
        && base.branch == plan.base.branch
        && base.commit == plan.base.commit
        && !git::operation_in_progress(&base.path)?)
}

/// Recheck from the main worktree before each deletion. Git's non-force remove
/// performs its own final dirty/locked checks; never recursively delete by hand.
pub fn execute(plan: &mut PrunePlan) -> GroveResult<()> {
    for i in 0..plan.entries.len() {
        if plan.entries[i].state != PruneState::Candidate {
            continue;
        }
        let wts = git::parse_worktree_list(&plan.base.path)?;
        if !base_unchanged(plan, &wts)? {
            return Err(GroveError::GitError(
                "prune base changed since preview; run prune again".into(),
            ));
        }
        let entry = &mut plan.entries[i];
        let Some(current) = wts.iter().find(|w| w.path == entry.wt.path) else {
            entry.state = PruneState::Skipped;
            entry.reason = "worktree registration changed since preview".into();
            continue;
        };
        if current.branch != entry.wt.branch || current.commit != entry.wt.commit {
            entry.state = PruneState::Skipped;
            entry.reason = "worktree HEAD changed since preview".into();
            continue;
        }
        let (state, reason) = assessment(current, &plan.base, &plan.cwd, &wts);
        if state != PruneState::Candidate {
            entry.state = state;
            entry.reason = reason;
            continue;
        }
        match git::git_checked(
            &plan.base.path,
            &["worktree", "remove", "--", &entry.wt.path.to_string_lossy()],
        ) {
            Ok(_) => {
                entry.state = PruneState::Removed;
                entry.reason = "worktree removed; branch retained".into();
            }
            Err(error) => {
                entry.state = PruneState::Failed;
                entry.reason = error.to_string();
            }
        }
    }
    Ok(())
}
