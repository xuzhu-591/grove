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
