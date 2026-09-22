use console::{measure_text_width, pad_str, Alignment, Style, Term};
use grove_core::git::Worktree;
use grove_core::prune::{PruneEntry, PrunePlan, PruneState};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum Phase {
    Preview,
    Result,
}

pub fn print(plan: &PrunePlan, verbose: bool, phase: Phase, stderr: bool) {
    let term = if stderr {
        Term::stderr()
    } else {
        Term::stdout()
    };
    let color = if stderr {
        console::colors_enabled_stderr()
    } else {
        console::colors_enabled()
    };
    let text = render(
        &plan.base,
        &plan.entries,
        verbose,
        phase,
        term.size().1 as usize,
        color,
    );
    if stderr {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}

struct Display {
    text: String,
    width: usize,
    color: bool,
}

impl Display {
    fn blank(&mut self) {
        self.text.push('\n');
    }

    // Wrap complete values by display width, never truncate deletion targets.
    // Style each wrapped line afterwards so escape codes cannot be split.
    fn line(&mut self, indent: usize, text: &str, style: Style) {
        let indent = indent.min(self.width.saturating_sub(2));
        let available = self.width.saturating_sub(indent).max(2);
        let prefix = " ".repeat(indent);
        let mut line = String::new();
        for ch in visible(text).chars() {
            let next = format!("{line}{ch}");
            if !line.is_empty() && measure_text_width(&next) > available {
                self.text.push_str(&format!(
                    "{prefix}{}\n",
                    style.clone().force_styling(self.color).apply_to(&line)
                ));
                line.clear();
            }
            line.push(ch);
        }
        self.text.push_str(&format!(
            "{prefix}{}\n",
            style.force_styling(self.color).apply_to(line)
        ));
    }

    fn entries(&mut self, entries: &[&PruneEntry], style: Style, reasons: bool) {
        if entries.is_empty() {
            return;
        }
        let root = common_parent(entries);
        if let Some(root) = root {
            self.line(
                2,
                &format!("Directory: {}/", short_path(root).trim_end_matches('/')),
                Style::new().dim(),
            );
        }
        let rows: Vec<_> = entries
            .iter()
            .map(|e| {
                let dir = match root {
                    Some(root) => e.wt.path.strip_prefix(root).unwrap().display().to_string(),
                    None => short_path(&e.wt.path),
                };
                (visible(&e.wt.branch), visible(&dir))
            })
            .collect();
        let branch_width = rows
            .iter()
            .map(|(b, _)| measure_text_width(b))
            .max()
            .unwrap_or(0)
            .max(6);
        let dir_width = rows
            .iter()
            .map(|(_, d)| measure_text_width(d))
            .max()
            .unwrap_or(0)
            .max(9);
        let table = !reasons && branch_width + dir_width + 4 <= self.width;
        if table {
            self.line(
                2,
                &format!(
                    "{}  DIRECTORY",
                    pad_str("BRANCH", branch_width, Alignment::Left, None)
                ),
                Style::new().bold().dim(),
            );
        }
        for (index, ((branch, dir), entry)) in rows.iter().zip(entries).enumerate() {
            if table {
                self.line(
                    2,
                    &format!(
                        "{}  {dir}",
                        pad_str(branch, branch_width, Alignment::Left, None)
                    ),
                    style.clone(),
                );
            } else {
                self.line(2, branch, style.clone());
                self.line(4, &format!("path: {dir}"), Style::new().dim());
                if reasons {
                    self.line(4, &entry.reason, style.clone());
                }
                if index + 1 < entries.len() {
                    self.blank();
                }
            }
        }
    }

    fn skipped(&mut self, entries: &[&PruneEntry], verbose: bool) {
        if entries.is_empty() {
            return;
        }
        self.blank();
        self.line(
            0,
            &format!("Skipped ({})", entries.len()),
            Style::new().yellow().bold(),
        );
        if verbose {
            self.entries(entries, Style::new().yellow(), true);
        } else {
            let mut groups = BTreeMap::<&str, usize>::new();
            for entry in entries {
                *groups.entry(&entry.reason).or_default() += 1;
            }
            let mut groups: Vec<_> = groups.into_iter().collect();
            groups.sort_by(|(a, ac), (b, bc)| bc.cmp(ac).then(a.cmp(b)));
            for (reason, count) in groups {
                self.line(2, &format!("{count:>3}  {reason}"), Style::new().dim());
            }
            self.line(2, "Show skipped worktrees: --verbose", Style::new().dim());
        }
    }
}

fn visible(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn short_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        if let Ok(relative) = path.strip_prefix(Path::new(&home)) {
            return if relative.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", relative.display())
            };
        }
    }
    path.display().to_string()
}

fn common_parent<'a>(entries: &[&'a PruneEntry]) -> Option<&'a Path> {
    let mut parent = entries.first()?.wt.path.parent()?;
    for entry in entries {
        while !entry.wt.path.starts_with(parent) {
            parent = parent.parent()?;
        }
    }
    // Showing '/' as a common root makes unrelated paths harder to recognize.
    parent.parent().map(|_| parent)
}

fn render(
    base: &Worktree,
    entries: &[PruneEntry],
    verbose: bool,
    phase: Phase,
    width: usize,
    color: bool,
) -> String {
    let mut out = Display {
        text: String::new(),
        width: width.max(2),
        color,
    };
    let selected = |state| {
        entries
            .iter()
            .filter(|e| e.state == state)
            .collect::<Vec<_>>()
    };
    let candidates = selected(PruneState::Candidate);
    let removed = selected(PruneState::Removed);
    let skipped = selected(PruneState::Skipped);
    let failed = selected(PruneState::Failed);
    out.line(
        0,
        &format!(
            "Prune · {} @ {}",
            base.branch,
            &base.commit[..7.min(base.commit.len())]
        ),
        Style::new().bold(),
    );
    out.blank();
    match phase {
        Phase::Preview => {
            if candidates.is_empty() {
                out.line(0, "Nothing to prune.", Style::new().bold());
            } else {
                out.line(
                    0,
                    &format!("Ready to remove ({})", candidates.len()),
                    Style::new().cyan().bold(),
                );
                out.entries(&candidates, Style::new().cyan(), false);
            }
        }
        Phase::Result => {
            out.line(
                0,
                &format!(
                    "Removed {} · Skipped {} · Failed {}",
                    removed.len(),
                    skipped.len(),
                    failed.len()
                ),
                if failed.is_empty() {
                    Style::new().bold()
                } else {
                    Style::new().red().bold()
                },
            );
            if !failed.is_empty() {
                out.blank();
                out.line(
                    0,
                    &format!("Failed ({})", failed.len()),
                    Style::new().red().bold(),
                );
                out.entries(&failed, Style::new().red(), true);
            }
            if !removed.is_empty() {
                out.blank();
                out.line(
                    0,
                    &format!("Removed ({})", removed.len()),
                    Style::new().green().bold(),
                );
                out.entries(&removed, Style::new().green(), false);
            }
            if !candidates.is_empty() {
                out.blank();
                out.line(
                    0,
                    &format!("Not attempted ({})", candidates.len()),
                    Style::new().yellow().bold(),
                );
                out.entries(&candidates, Style::new().yellow(), false);
            }
        }
    }
    out.skipped(&skipped, verbose);
    if matches!(phase, Phase::Preview) && !candidates.is_empty() {
        out.blank();
        out.line(
            0,
            "Ignored files will be deleted too.",
            Style::new().yellow(),
        );
        out.line(
            0,
            "Local and remote branches are retained.",
            Style::new().dim(),
        );
    }
    out.blank();
    out.text
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::strip_ansi_codes;

    fn entry(branch: &str, dir: &str, state: PruneState, reason: &str) -> PruneEntry {
        PruneEntry {
            wt: Worktree {
                branch: branch.into(),
                path: dir.into(),
                commit: "123456789abcdef".into(),
                prunable: false,
                locked: false,
                bare: false,
            },
            state,
            reason: reason.into(),
        }
    }

    fn base() -> Worktree {
        entry("main", "/repo", PruneState::Skipped, "main worktree").wt
    }

    #[test]
    fn preview_prioritizes_candidates_and_groups_skips_without_hiding_targets() {
        let entries = vec![
            entry(
                "feat/dirty-one",
                "/worktrees/dirty-one",
                PruneState::Skipped,
                "uncommitted or untracked changes",
            ),
            entry(
                "feat/done",
                "/worktrees/actual-directory",
                PruneState::Candidate,
                "merged and clean",
            ),
            entry(
                "feat/dirty-two",
                "/worktrees/dirty-two",
                PruneState::Skipped,
                "uncommitted or untracked changes",
            ),
        ];
        let text = render(&base(), &entries, false, Phase::Preview, 100, false);
        assert!(text.find("feat/done").unwrap() < text.find("Skipped (2)").unwrap());
        assert!(text.contains("Directory: /worktrees/"));
        assert!(text.contains("actual-directory"));
        assert!(!text.contains("feat/dirty-one"));
        assert!(!text.contains("feat/dirty-two"));
        assert_eq!(text.matches("uncommitted or untracked changes").count(), 1);
        assert!(text.contains("2  uncommitted"));
        assert!(!text.contains('\t'));
        assert!(!text.contains("123456789abcdef"));
        assert!(text.contains("main @ 1234567"));
    }

    #[test]
    fn narrow_unicode_output_fits_without_truncating_paths_or_branches() {
        let branch = "feat/支持中文的较长分支名称-with-a-long-suffix";
        let entries = vec![entry(
            branch,
            "/worktrees/实际目录-long-directory-name",
            PruneState::Candidate,
            "",
        )];
        for width in [40, 60, 80, 120] {
            let text = render(&base(), &entries, false, Phase::Preview, width, true);
            let plain = strip_ansi_codes(&text);
            assert!(
                plain.lines().all(|l| measure_text_width(l) <= width),
                "{width}: {plain}"
            );
            let joined: String = plain.lines().map(str::trim_start).collect();
            assert!(joined.contains(branch), "{joined}");
            assert!(joined.contains("实际目录-long-directory-name"));
            assert!(!plain.contains("..."));
        }
    }

    #[test]
    fn wide_table_uses_display_width_for_aligned_unicode_columns() {
        let entries = vec![
            entry("feat/中文", "/worktrees/one", PruneState::Candidate, ""),
            entry("feat/ascii", "/worktrees/two", PruneState::Candidate, ""),
        ];
        let text = render(&base(), &entries, false, Phase::Preview, 120, false);
        let rows: Vec<_> = text.lines().filter(|l| l.starts_with("  feat/")).collect();
        let one = measure_text_width(rows[0].split("one").next().unwrap());
        let two = measure_text_width(rows[1].split("two").next().unwrap());
        assert_eq!(one, two);
        assert!(text.contains("BRANCH"));
    }

    #[test]
    fn verbose_shows_skipped_paths_and_full_reasons_and_keeps_unrelated_paths_absolute() {
        let entries = vec![
            entry(
                "feat/one",
                "/alpha/one",
                PruneState::Skipped,
                "unable to verify: index damaged",
            ),
            entry(
                "feat/two",
                "/beta/two",
                PruneState::Skipped,
                "locked worktree",
            ),
        ];
        let text = render(&base(), &entries, true, Phase::Preview, 120, false);
        assert!(text.contains("Nothing to prune."));
        assert!(!text.contains("Ignored files"));
        assert!(!text.contains("Directory:"));
        for value in [
            "feat/one",
            "/alpha/one",
            "index damaged",
            "feat/two",
            "/beta/two",
            "locked worktree",
        ] {
            assert!(text.contains(value), "{text}");
        }
    }

    #[test]
    fn results_highlight_failures_preserve_full_reason_and_label_unattempted_entries() {
        let reason =
            "fatal: cannot remove a worktree containing submodules; worktree remains intact";
        let entries = vec![
            entry(
                "feat/removed",
                "/worktrees/removed",
                PruneState::Removed,
                "removed",
            ),
            entry(
                "feat/failed",
                "/worktrees/failed",
                PruneState::Failed,
                reason,
            ),
            entry(
                "feat/pending",
                "/worktrees/pending",
                PruneState::Candidate,
                "merged",
            ),
        ];
        let text = render(&base(), &entries, false, Phase::Result, 60, false);
        assert!(text.contains("Removed 1 · Skipped 0 · Failed 1"));
        assert!(text.find("feat/failed").unwrap() < text.find("feat/removed").unwrap());
        let joined: String = text.lines().map(str::trim_start).collect();
        assert!(joined.contains(reason));
        assert!(text.contains("Not attempted (1)"));
        assert!(!text.contains("Ready to remove"));
    }

    #[test]
    fn human_output_escapes_control_characters_in_paths() {
        let entries = vec![entry(
            "feat/safe",
            "/worktrees/tab\tnewline\nescape\u{1b}[31m",
            PruneState::Candidate,
            "",
        )];
        let text = render(&base(), &entries, false, Phase::Preview, 120, false);
        assert!(text.contains("tab\\tnewline\\nescape\\u{1b}[31m"), "{text}");
        assert!(!text.contains('\t'));
        assert!(!text.contains('\u{1b}'));
    }
}
