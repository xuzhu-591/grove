mod helpers;

use helpers::TestRepo;

#[test]
fn test_add_local_branch() {
    let repo = TestRepo::new();
    repo.create_branch("feat/test-add");
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "add", "feat/test-add"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        !stdout.trim().is_empty(),
        "expected worktree path in stdout"
    );
    let wt_dir = stdout.trim().to_string();
    assert!(std::path::Path::new(&wt_dir).is_dir());
}

#[test]
fn test_add_create_new_branch() {
    let repo = TestRepo::new();
    let (code, stdout, stderr) = repo.run_grove(&["--plain", "add", "feat/new-branch", "--create"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let wt_dir = stdout.trim().to_string();
    assert!(std::path::Path::new(&wt_dir).is_dir());
    assert!(
        wt_dir.contains("feat-new-branch"),
        "worktree path should contain safe branch name, got: {wt_dir}"
    );
}

#[test]
fn test_add_non_existent_branch_fails() {
    let repo = TestRepo::new();
    let (code, _, _) = repo.run_grove(&["--plain", "add", "nonexistent-branch"]);
    assert_ne!(code, 0, "should fail for non-existent branch");
}
