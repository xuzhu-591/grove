mod helpers;

use grove_core::prune::{self, PruneState};
use helpers::TestRepo;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().into()
}

fn add(repo: &TestRepo, branch: &str) -> PathBuf {
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "add", branch, "--create"]);
    assert_eq!(code, 0, "{stderr}");
    PathBuf::from(stdout.trim())
}

fn assert_skip(plan: &prune::PrunePlan, branch: &str, reason: &str) {
    let e = plan.entries.iter().find(|e| e.wt.branch == branch).unwrap();
    assert_eq!(e.state, PruneState::Skipped);
    assert!(e.reason.contains(reason), "{}", e.reason);
}

#[test]
fn dry_run_preserves_then_yes_removes_ignored_files_and_preserves_branches_and_symlink_target() {
    let repo = TestRepo::new();
    std::fs::write(repo.work_repo().join(".gitignore"), "ignored/\nexternal\n").unwrap();
    git(repo.work_repo(), &["add", ".gitignore"]);
    git(
        repo.work_repo(),
        &["commit", "-m", "ignore build artifacts"],
    );
    git(repo.work_repo(), &["push", "origin", "main"]);
    let path = add(&repo, "feat/merged");
    git(&path, &["push", "origin", "feat/merged"]);
    std::fs::create_dir(path.join("ignored")).unwrap();
    std::fs::write(path.join("ignored/local.txt"), "local data").unwrap();
    let external = repo.temp_dir.path().join("external");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("keep.txt"), "preserve").unwrap();
    std::os::unix::fs::symlink(&external, path.join("external")).unwrap();
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "prune", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\tcandidate\t"));
    assert!(stderr.contains("Ignored files"));
    assert!(path.join("ignored/local.txt").exists());
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "prune", "--yes"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("\tremoved\t"));
    assert!(!path.exists());
    assert!(external.join("keep.txt").exists());
    git(
        repo.work_repo(),
        &["show-ref", "--verify", "refs/heads/feat/merged"],
    );
    assert!(git(
        repo.work_repo(),
        &["ls-remote", "--heads", "origin", "feat/merged"]
    )
    .contains("refs/heads/feat/merged"));
}

#[test]
fn prune_skips_dirty_locked_detached_unmerged_missing_and_in_progress() {
    let repo = TestRepo::new();
    for branch in [
        "feat/unstaged",
        "feat/staged",
        "feat/untracked",
        "feat/locked",
        "feat/detached",
        "feat/unmerged",
        "feat/missing",
        "feat/operation",
    ] {
        let path = add(&repo, branch);
        match branch {
            "feat/unstaged" => std::fs::write(path.join("file.txt"), "edit").unwrap(),
            "feat/staged" => {
                std::fs::write(path.join("file.txt"), "edit").unwrap();
                git(&path, &["add", "file.txt"]);
            }
            "feat/untracked" => {
                std::fs::write(path.join("untracked"), "data").unwrap();
            }
            "feat/locked" => {
                git(
                    repo.work_repo(),
                    &["worktree", "lock", path.to_str().unwrap()],
                );
            }
            "feat/detached" => {
                git(&path, &["checkout", "--detach"]);
            }
            "feat/unmerged" => {
                std::fs::write(path.join("file.txt"), "commit").unwrap();
                git(&path, &["commit", "-am", "unmerged"]);
            }
            "feat/missing" => std::fs::remove_dir_all(path).unwrap(),
            "feat/operation" => {
                let marker = git(&path, &["rev-parse", "--git-path", "MERGE_HEAD"]);
                std::fs::write(path.join(marker), repo.head()).unwrap();
            }
            _ => unreachable!(),
        }
    }
    let plan = prune::prepare(repo.work_repo()).unwrap();
    for branch in ["feat/unstaged", "feat/staged", "feat/untracked"] {
        assert_skip(&plan, branch, "changes");
    }
    assert_skip(&plan, "main", "main worktree");
    assert_skip(&plan, "feat/locked", "locked");
    assert_skip(&plan, "(detached)", "no branch");
    assert_skip(&plan, "feat/unmerged", "not merged");
    assert_skip(&plan, "feat/missing", "missing");
    assert_skip(&plan, "feat/operation", "operation");
}

#[test]
fn prune_preserves_current_worktree_when_called_from_subdirectory() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/current");
    let child = path.join("subdir");
    std::fs::create_dir(&child).unwrap();
    let mut plan = prune::prepare(&child).unwrap();
    assert_skip(&plan, "feat/current", "current worktree");
    prune::execute(&mut plan).unwrap();
    assert!(path.exists());
}

#[test]
fn prune_rechecks_changes_after_preview() {
    let repo = TestRepo::new();
    let dirty = add(&repo, "feat/dirty");
    let changed = add(&repo, "feat/changed");
    let locked = add(&repo, "feat/locked");
    let operation = add(&repo, "feat/operation");
    let mut plan = prune::prepare(repo.work_repo()).unwrap();
    std::fs::write(dirty.join("local.txt"), "data").unwrap();
    git(&changed, &["commit", "--allow-empty", "-m", "new HEAD"]);
    git(
        repo.work_repo(),
        &["worktree", "lock", locked.to_str().unwrap()],
    );
    let marker = git(&operation, &["rev-parse", "--git-path", "rebase-merge"]);
    std::fs::create_dir(operation.join(marker)).unwrap();
    prune::execute(&mut plan).unwrap();
    assert_skip(&plan, "feat/dirty", "changes");
    assert_skip(&plan, "feat/changed", "HEAD changed");
    assert_skip(&plan, "feat/locked", "locked");
    assert_skip(&plan, "feat/operation", "operation");
    for path in [dirty, changed, locked, operation] {
        assert!(path.exists());
    }
}

#[test]
fn prune_aborts_if_base_changes_after_preview() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/keep");
    let mut plan = prune::prepare(repo.work_repo()).unwrap();
    git(
        repo.work_repo(),
        &["commit", "--allow-empty", "-m", "base moved"],
    );
    assert!(prune::execute(&mut plan)
        .unwrap_err()
        .to_string()
        .contains("base changed"));
    assert!(path.exists());
}

#[test]
fn prune_rejects_unreadable_status_and_replaced_directory() {
    let repo = TestRepo::new();
    let broken = add(&repo, "feat/broken");
    let replaced = add(&repo, "feat/replaced");
    let mut plan = prune::prepare(repo.work_repo()).unwrap();
    let index = git(&broken, &["rev-parse", "--git-path", "index"]);
    std::fs::write(broken.join(index), "corrupt index").unwrap();
    std::fs::remove_file(replaced.join(".git")).unwrap();
    git(&replaced, &["init", "-q"]);
    prune::execute(&mut plan).unwrap();
    assert_skip(&plan, "feat/broken", "unable to verify");
    assert_skip(&plan, "feat/replaced", "identity changed");
    assert!(broken.exists());
    assert!(replaced.exists());
}

#[test]
fn prune_refreshes_base_before_deciding_merged() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/remote-merged");
    git(&path, &["commit", "--allow-empty", "-m", "remote merge"]);
    git(&path, &["push", "origin", "HEAD:main"]);
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "prune", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(repo.head(), git(&path, &["rev-parse", "HEAD"]));
    assert!(stdout.contains("\tcandidate\t"));
    assert!(path.exists());
}

#[test]
fn prune_rejects_fetch_failure_no_upstream_dirty_behind_and_divergence() {
    for case in ["fetch", "upstream", "dirty", "diverged", "gone-upstream"] {
        let repo = TestRepo::new();
        let path = add(&repo, "feat/keep");
        match case {
            "fetch" => repo.set_origin_url(&repo.temp_dir.path().join("absent")),
            "upstream" => {
                git(repo.work_repo(), &["branch", "--unset-upstream"]);
            }
            "gone-upstream" => {
                git(
                    repo.work_repo(),
                    &["config", "branch.main.merge", "refs/heads/missing"],
                );
            }
            "dirty" => {
                repo.commit_and_push_remote_main("remote.txt", "remote");
                std::fs::write(repo.work_repo().join("local.txt"), "local").unwrap();
            }
            "diverged" => {
                repo.commit_and_push_remote_main("remote.txt", "remote");
                git(
                    repo.work_repo(),
                    &["commit", "--allow-empty", "-m", "local"],
                );
            }
            _ => unreachable!(),
        }
        let (code, _, stderr) = repo.run_grove(&["--plain", "prune", "--yes"]);
        assert_ne!(code, 0, "case {case}: {stderr}");
        assert!(path.exists());
    }
}

#[test]
fn prune_requires_explicit_noninteractive_action() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/keep");
    let (code, _, stderr) = repo.run_grove(&["--plain", "prune"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("--dry-run"));
    assert!(path.exists());
}

#[test]
fn prune_supports_paths_with_spaces_tabs_and_newlines() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/path");
    let renamed = path.with_file_name("space tab\tnewline\nworktree");
    git(
        repo.work_repo(),
        &[
            "worktree",
            "move",
            path.to_str().unwrap(),
            renamed.to_str().unwrap(),
        ],
    );
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "prune", "--yes"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.lines().count(), 2);
    assert!(stdout.contains("space tab\\tnewline\\nworktree"));
    assert!(!renamed.exists());
}

#[test]
fn prune_reports_git_refusal_and_continues_with_other_candidates() {
    let repo = TestRepo::new();
    repo.create_branch("feat/clean");
    let module = repo.temp_dir.path().join("module");
    std::fs::create_dir(&module).unwrap();
    git(&module, &["init", "-q"]);
    git(&module, &["config", "user.name", "Test"]);
    git(&module, &["config", "user.email", "test@example.invalid"]);
    git(&module, &["commit", "--allow-empty", "-m", "module"]);
    git(
        repo.work_repo(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            module.to_str().unwrap(),
            "sub",
        ],
    );
    git(repo.work_repo(), &["commit", "-am", "add submodule"]);
    git(repo.work_repo(), &["push", "origin", "main"]);
    let blocked = add(&repo, "feat/blocked");
    git(
        &blocked,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
        ],
    );
    // An older branch with no submodule is also merged and remains removable.
    let (code, path, err) = repo.run_grove(&["--plain", "add", "feat/clean"]);
    assert_eq!(code, 0, "{err}");
    let clean = PathBuf::from(path.trim());
    let (code, out, err) = repo.run_grove(&["--plain", "prune", "--yes"]);
    assert_ne!(code, 0, "{err}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("feat/blocked\t") && l.contains("\tfailed\t")),
        "{out}"
    );
    assert!(
        out.lines()
            .any(|l| l.starts_with("feat/clean\t") && l.contains("\tremoved\t")),
        "{out}"
    );
    assert!(blocked.exists());
    assert!(!clean.exists());
}

#[test]
fn human_preview_groups_skips_and_verbose_expands_without_changing_plain_output() {
    let repo = TestRepo::new();
    let clean = add(&repo, "feat/candidate");
    let dirty = add(&repo, "feat/dirty");
    std::fs::write(dirty.join("local.txt"), "keep").unwrap();
    let (code, human, err) = repo.run_grove(&["prune", "--dry-run"]);
    assert_eq!(code, 0, "{err}");
    assert!(human.contains("Ready to remove (1)"), "{human}");
    assert!(human.contains("feat/candidate"));
    assert!(human.contains("Skipped (2)"));
    assert!(!human.contains("feat/dirty"));
    assert!(!human.contains('\t'));
    let (code, verbose, err) = repo.run_grove(&["prune", "--dry-run", "--verbose"]);
    assert_eq!(code, 0, "{err}");
    assert!(verbose.contains("feat/dirty"));
    assert!(verbose.contains("uncommitted or untracked changes"));
    let (code, plain, err) = repo.run_grove(&["--plain", "prune", "--dry-run"]);
    assert_eq!(code, 0, "{err}");
    let (code, plain_verbose, err) =
        repo.run_grove(&["--plain", "prune", "--dry-run", "--verbose"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(plain, plain_verbose);
    assert_eq!(plain.lines().count(), 3);
    assert!(plain.lines().all(|l| l.split('\t').count() == 4));
    assert!(plain.contains("\tcandidate\tmerged and clean; ignored files will be removed"));
    assert!(clean.exists());
    assert!(dirty.join("local.txt").exists());
}

#[test]
fn human_execution_shows_results_and_empty_preview_is_explicit() {
    let repo = TestRepo::new();
    let path = add(&repo, "feat/done");
    let (code, result, err) = repo.run_grove(&["prune", "--yes"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        result.contains("Removed 1 · Skipped 1 · Failed 0"),
        "{result}"
    );
    assert!(result.contains("feat/done"));
    assert!(!path.exists());
    let (code, preview, err) = repo.run_grove(&["prune", "--dry-run"]);
    assert_eq!(code, 0, "{err}");
    assert!(preview.contains("Nothing to prune."));
    assert!(!preview.contains("Ready to remove"));
    assert!(!preview.contains("Ignored files"));
}
