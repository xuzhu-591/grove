mod helpers;

use helpers::TestRepo;

#[test]
fn test_switch_outputs_path() {
    let repo = TestRepo::new();
    repo.create_branch("feat/switch-test");
    repo.run_grove(&["--plain", "add", "feat/switch-test"]);

    let (code, stdout, _) = repo.run_grove(&["--plain", "switch", "feat/switch-test"]);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
    assert!(std::path::Path::new(stdout.trim()).is_dir());
}

#[test]
fn test_switch_non_existent_fails() {
    let repo = TestRepo::new();
    let (code, _, _) = repo.run_grove(&["--plain", "switch", "no-such-branch"]);
    assert_ne!(code, 0);
}
