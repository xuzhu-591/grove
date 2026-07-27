mod helpers;

use helpers::TestRepo;

#[test]
fn test_list_plain_output_format() {
    let repo = TestRepo::new();
    let (code, stdout, _) = repo.run_grove(&["--plain", "list"]);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
    let line = stdout.lines().next().unwrap();
    assert!(line.contains('\t'), "expected TSV output, got: {line}");
    assert!(line.contains("main"), "expected main branch in output");
}

#[test]
fn test_list_after_adding_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feat/list-test");
    repo.run_grove(&["--plain", "add", "feat/list-test"]);

    let (code, stdout, _) = repo.run_grove(&["--plain", "list"]);
    assert_eq!(code, 0);
    let lines: Vec<_> = stdout.lines().collect();
    assert!(
        lines.len() >= 2,
        "expected at least 2 worktrees, got {}",
        lines.len()
    );
    assert!(lines.iter().any(|l| l.contains("feat/list-test")));
}

#[test]
fn test_list_fast_forwards_clean_main_worktree() {
    let repo = TestRepo::new();
    repo.commit_and_push_remote_main("remote.txt", "remote\n");
    let before = repo.head();

    let (code, stdout, stderr) = repo.run_grove(&["--plain", "list"]);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_ne!(
        repo.head(),
        before,
        "main worktree should be fast-forwarded"
    );
    assert_eq!(
        std::fs::read_to_string(repo.work_repo().join("remote.txt")).unwrap(),
        "remote\n"
    );
    let main_line = stdout
        .lines()
        .find(|line| line.starts_with("main\t"))
        .unwrap();
    assert!(
        main_line.contains("behind=0"),
        "expected synced main: {main_line}"
    );
    assert!(stderr.is_empty(), "unexpected warning: {stderr}");
}

#[test]
fn test_list_warns_without_updating_diverged_main_worktree() {
    let repo = TestRepo::new();
    repo.commit_on_branch("main", "local.txt", "local\n");
    let before = repo.head();
    repo.commit_and_push_remote_main("remote.txt", "remote\n");

    let (code, stdout, stderr) = repo.run_grove(&["--plain", "list"]);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(repo.head(), before, "diverged main must not be updated");
    assert!(
        stderr.contains("branch diverged from upstream"),
        "expected divergence warning: {stderr}"
    );
    let main_line = stdout
        .lines()
        .find(|line| line.starts_with("main\t"))
        .unwrap();
    assert!(
        main_line.contains("ahead=1"),
        "expected local commit: {main_line}"
    );
    assert!(
        main_line.contains("behind=1"),
        "expected remote commit: {main_line}"
    );
}

#[test]
fn test_list_warns_without_updating_dirty_main_worktree() {
    let repo = TestRepo::new();
    repo.commit_and_push_remote_main("remote.txt", "remote\n");
    std::fs::write(repo.work_repo().join("local.txt"), "local\n").unwrap();
    let before = repo.head();

    let (code, stdout, stderr) = repo.run_grove(&["--plain", "list"]);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(repo.head(), before, "dirty main must not be updated");
    assert!(
        stderr.contains("local changes are present"),
        "expected dirty-worktree warning: {stderr}"
    );
    assert!(
        !repo.work_repo().join("remote.txt").exists(),
        "remote commit must not be applied to a dirty worktree"
    );
    let main_line = stdout
        .lines()
        .find(|line| line.starts_with("main\t"))
        .unwrap();
    assert!(
        main_line.contains("behind=1"),
        "expected remote commit: {main_line}"
    );
}

#[test]
fn test_list_warns_and_uses_cached_refs_when_fetch_fails() {
    let repo = TestRepo::new();
    let missing_origin = repo.temp_dir.path().join("missing-origin");
    repo.set_origin_url(&missing_origin);

    let (code, stdout, stderr) = repo.run_grove(&["--plain", "list"]);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains("Unable to refresh remotes before listing worktrees"),
        "expected fetch warning: {stderr}"
    );
    assert!(stdout.lines().any(|line| line.starts_with("main\t")));
}

#[test]
fn test_list_shows_merge_status() {
    let repo = TestRepo::new();

    // Branch with no new commits: ancestor of main -> merged.
    repo.create_branch("feat/merged");
    repo.run_grove(&["--plain", "add", "feat/merged"]);

    // Branch with a commit not in main -> unmerged.
    repo.create_branch("feat/wip");
    repo.commit_on_branch("feat/wip", "wip.txt", "wip\n");
    repo.checkout("main");
    repo.run_grove(&["--plain", "add", "feat/wip"]);

    // Branch merged into main via a merge commit -> merged.
    repo.create_branch("feat/done");
    repo.commit_on_branch("feat/done", "done.txt", "done\n");
    repo.merge_into_main("feat/done");
    repo.run_grove(&["--plain", "add", "feat/done"]);

    let (code, stdout, _) = repo.run_grove(&["--plain", "list"]);
    assert_eq!(code, 0);

    let main_line = stdout.lines().find(|l| l.starts_with("main\t")).unwrap();
    assert!(
        main_line.contains("merged=-"),
        "main worktree should be N/A, got: {main_line}"
    );

    let merged_line = stdout
        .lines()
        .find(|l| l.starts_with("feat/merged\t"))
        .unwrap();
    assert!(
        merged_line.contains("merged=yes"),
        "branch with no new commits should be merged, got: {merged_line}"
    );

    let wip_line = stdout
        .lines()
        .find(|l| l.starts_with("feat/wip\t"))
        .unwrap();
    assert!(
        wip_line.contains("merged=no"),
        "branch with unmerged commits should not be merged, got: {wip_line}"
    );

    let done_line = stdout
        .lines()
        .find(|l| l.starts_with("feat/done\t"))
        .unwrap();
    assert!(
        done_line.contains("merged=yes"),
        "merged branch should be merged, got: {done_line}"
    );
}
