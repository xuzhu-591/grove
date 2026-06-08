mod helpers;

use helpers::TestRepo;

#[test]
fn test_remove_clean_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feat/clean");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/clean"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    repo.checkout("main");

    let (code, _, _) = repo.run_grove(&["--plain", "remove", "feat/clean"]);
    assert_eq!(code, 0);
    assert!(
        !std::path::Path::new(&wt_dir).exists(),
        "worktree dir should be removed"
    );
}

#[test]
fn test_cannot_remove_main_worktree() {
    let repo = TestRepo::new();
    let (code, _, _stderr) = repo.run_grove(&["--plain", "remove", "main"]);
    assert_ne!(code, 0, "should not be able to remove main worktree");
}
