use console::{pad_str, style, Alignment};
use grove_core::git::WorktreeStatus;
use grove_core::worktree::{MergeState, WorktreeEntry};
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

fn merged_plain_value(state: MergeState) -> &'static str {
    match state {
        MergeState::Merged => "yes",
        MergeState::Unmerged => "no",
        MergeState::NotApplicable => "-",
        MergeState::Unknown => "N/A",
    }
}

pub fn format_list_entry_plain(entry: &WorktreeEntry) -> String {
    let values = match &entry.status {
        Ok(s) => [s.staged, s.modified, s.untracked, s.ahead, s.behind].map(|n| n.to_string()),
        Err(_) => std::array::from_fn(|_| "N/A".to_string()),
    };
    format!(
        "{}\t{}\t{}\tstaged={}\tmodified={}\tuntracked={}\tahead={}\tbehind={}\tmerged={}",
        entry.wt.branch,
        entry.wt.path.display(),
        &entry.wt.commit[..7.min(entry.wt.commit.len())],
        values[0],
        values[1],
        values[2],
        values[3],
        values[4],
        merged_plain_value(entry.merged),
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
        .clamp(3, 80);

    // Width of the longest merge label ("unmerged"); keeps STATUS aligned.
    let max_merged = 8;

    // Header: pad first, then wrap with style (avoids ANSI width issues)
    println!(
        "  {}  {}  {}  {}  {}",
        style(pad_str("BRANCH", max_branch, Alignment::Left, None)).bold(),
        style(pad_str("DIR", max_dir, Alignment::Left, None)).bold(),
        style(pad_str("COMMIT", 7, Alignment::Left, None)).bold(),
        style(pad_str("MERGED", max_merged, Alignment::Left, None)).bold(),
        style(pad_str("STATUS", 0, Alignment::Left, None)).bold(),
    );

    for entry in entries {
        let marker = if entry.is_main { "*" } else { " " };
        let short_dir = grove_core::path::short_path(&entry.wt.path);
        let mut display_dir = short_dir.clone();
        if display_dir.len() > max_dir {
            display_dir = format!("{}...", &display_dir[..max_dir.saturating_sub(3)]);
        }

        // Pad plain text, then apply ANSI styles — avoids color codes breaking alignment
        let branch_col = style(pad_str(&entry.wt.branch, max_branch, Alignment::Left, None)).cyan();
        let dir_col = pad_str(&display_dir, max_dir, Alignment::Left, None);
        let commit_col = style(pad_str(
            &entry.wt.commit[..7.min(entry.wt.commit.len())],
            7,
            Alignment::Left,
            None,
        ))
        .dim();
        let merged_col = match entry.merged {
            MergeState::Merged => {
                style(pad_str("merged", max_merged, Alignment::Left, None)).green()
            }
            MergeState::Unmerged => {
                style(pad_str("unmerged", max_merged, Alignment::Left, None)).yellow()
            }
            MergeState::Unknown => style(pad_str("N/A", max_merged, Alignment::Left, None)).red(),
            MergeState::NotApplicable => {
                style(pad_str("-", max_merged, Alignment::Left, None)).dim()
            }
        };
        let status_col = match &entry.status {
            Ok(status) => format_status_human(status),
            Err(_) => style("N/A").red().to_string(),
        };

        println!("{marker} {branch_col}  {dir_col}  {commit_col}  {merged_col}  {status_col}");
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

fn prune_row(entry: &grove_core::prune::PruneEntry) -> String {
    // Escape control characters so each TSV record remains one physical line.
    fn escape(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }
    format!(
        "{}\t{}\t{}\t{}",
        escape(&entry.wt.branch),
        escape(&entry.wt.path.to_string_lossy()),
        entry.state.as_str(),
        escape(&entry.reason)
    )
}

pub fn preview_prune(plan: &grove_core::prune::PrunePlan) {
    eprintln!("BRANCH\tDIR\tRESULT\tREASON");
    for entry in &plan.entries {
        eprintln!("{}", prune_row(entry));
    }
}

pub fn print_prune(plan: &grove_core::prune::PrunePlan, plain: bool) {
    use grove_core::prune::PruneState;
    if !plain {
        println!("BRANCH\tDIR\tRESULT\tREASON");
    }
    for entry in &plan.entries {
        println!("{}", prune_row(entry));
    }
    let count = |state| plan.entries.iter().filter(|e| e.state == state).count();
    info(&format!(
        "Prune: {} candidate, {} removed, {} skipped, {} failed",
        count(PruneState::Candidate),
        count(PruneState::Removed),
        count(PruneState::Skipped),
        count(PruneState::Failed)
    ));
}
