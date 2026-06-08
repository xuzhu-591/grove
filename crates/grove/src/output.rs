use console::style;
use grove_core::git::WorktreeStatus;
use grove_core::worktree::WorktreeEntry;
use std::path::Path;

pub fn format_status_human(status: &WorktreeStatus) -> String {
    if status.staged == 0
        && status.modified == 0
        && status.untracked == 0
        && status.ahead == 0
        && status.behind == 0
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
        parts.push(style(format!("{}", status.ahead)).cyan().to_string());
    }
    if status.behind > 0 {
        parts.push(style(format!("{}", status.behind)).magenta().to_string());
    }
    parts.join(" ")
}

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

pub fn print_list_plain(entries: &[WorktreeEntry]) {
    for entry in entries {
        println!("{}", format_list_entry_plain(entry));
    }
}

pub fn emit_cd(path: &Path, plain: bool) {
    if let Ok(file) = std::env::var("GROVE_CD_FILE") {
        let _ = std::fs::write(&file, path.display().to_string());
    } else if plain {
        println!("{}", path.display());
    }
}

pub fn info(msg: &str) {
    eprintln!("{}", style(msg).green());
}

pub fn warn(msg: &str) {
    eprintln!("{}", style(msg).yellow());
}

pub fn error(msg: &str) {
    eprintln!("{}", style(msg).red());
}
