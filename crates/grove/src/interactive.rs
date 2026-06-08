use anyhow::{Context, Result};
use inquire::{Confirm, Select, Text};
use std::path::Path;

pub enum AddAction {
    ExistingBranch(String),
    NewBranch(String),
    RemoteBranch(String),
}

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

    let branch = selected.split_whitespace().next().unwrap_or("").to_string();
    Ok(branch)
}

pub fn remove_interactive(dir: &Path) -> Result<String> {
    let wts = grove_core::git::parse_worktree_list(dir)
        .context("failed to list worktrees")?;

    if wts.len() <= 1 {
        anyhow::bail!("no removable worktrees (main worktree cannot be removed)");
    }

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

pub fn cache_interactive() -> Result<String> {
    let action = Select::new("Cache 操作", vec!["link", "status", "unlink"]).prompt()?;
    Ok(action.to_string())
}
