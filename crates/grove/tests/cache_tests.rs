mod helpers;

use helpers::TestRepo;
use std::process::Command;

fn git_add_commit(dir: &std::path::Path, msg: &str) {
    let _ = Command::new("git").args(["add", "."]).current_dir(dir).output();
    let _ = Command::new("git").args(["commit", "-m", msg]).current_dir(dir).output();
}

#[test]
fn test_cache_link_creates_symlinks() {
    let repo = TestRepo::new();
    let work_repo = repo.work_repo();

    std::fs::create_dir_all(work_repo.join("node_modules")).unwrap();
    std::fs::create_dir_all(work_repo.join("packages/pkg-a/node_modules")).unwrap();
    std::fs::create_dir_all(work_repo.join("packages/pkg-b/node_modules")).unwrap();

    std::fs::write(
        work_repo.join("grove.toml"),
        r#"
[cache]
rules = ["node_modules", "packages/*/node_modules"]
"#,
    )
    .unwrap();
    git_add_commit(work_repo, "add grove.toml");

    repo.create_branch("feat/cache-test");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/cache-test"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();
    assert!(
        std::fs::symlink_metadata(format!("{wt_dir}/node_modules"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        std::fs::symlink_metadata(format!("{wt_dir}/packages/pkg-a/node_modules"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn test_cache_unlink_removes_symlinks() {
    let repo = TestRepo::new();
    let work_repo = repo.work_repo();

    std::fs::create_dir_all(work_repo.join("node_modules")).unwrap();
    std::fs::write(
        work_repo.join("grove.toml"),
        r#"
[cache]
rules = ["node_modules"]
"#,
    )
    .unwrap();
    git_add_commit(work_repo, "add grove.toml and node_modules");

    repo.create_branch("feat/unlink-test");
    let (code, stdout, _) = repo.run_grove(&["--plain", "add", "feat/unlink-test"]);
    assert_eq!(code, 0);

    let wt_dir = stdout.trim().to_string();

    let grove_bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("grove");

    let output = std::process::Command::new(&grove_bin)
        .args(["--plain", "cache", "unlink"])
        .current_dir(&wt_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!std::path::Path::new(&format!("{wt_dir}/node_modules")).exists());
}
