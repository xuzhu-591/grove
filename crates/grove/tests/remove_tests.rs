mod helpers;

use helpers::TestRepo;
use std::process::Command;

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

#[test]
fn test_remove_prunable_worktree() {
    let repo = TestRepo::new();
    repo.create_branch("feat/prunable");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/prunable"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    repo.checkout("main");
    std::fs::remove_dir_all(&wt_dir).unwrap();

    let before = git_worktree_list(repo.work_repo());
    assert!(
        before.contains("prunable"),
        "expected prunable worktree before removal:\n{}",
        before
    );

    let (code, _, stderr) = repo.run_grove(&["--plain", "remove", "feat/prunable"]);
    assert_eq!(code, 0, "remove failed: {}", stderr);

    let after = git_worktree_list(repo.work_repo());
    assert!(
        !after.contains("feat/prunable"),
        "worktree should be pruned:\n{}",
        after
    );
}

fn git_worktree_list(dir: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).to_string()
}
