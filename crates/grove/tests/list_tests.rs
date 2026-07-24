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
