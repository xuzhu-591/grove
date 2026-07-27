use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
pub struct TestRepo {
    pub temp_dir: tempfile::TempDir,
    pub work_repo: PathBuf,
    bare_origin: PathBuf,
    grove_bin: PathBuf,
    home_dir: PathBuf,
    worktree_base: PathBuf,
}

#[allow(dead_code)]
impl TestRepo {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let home_dir = temp_dir.path().join("home");
        std::fs::create_dir(&home_dir).unwrap();

        let bare_origin = temp_dir.path().join("bare_origin");
        let work_repo = temp_dir.path().join("work_repo");

        std::env::set_var("HOME", &home_dir);
        std::env::set_var(
            "GROVE_WORKTREE_BASE",
            temp_dir.path().join("grove_worktrees"),
        );

        // Create bare origin
        run_git_init_bare(&bare_origin);

        // Create initial content
        let tmp_clone = temp_dir.path().join("tmp_clone");
        git_clone(&bare_origin, &tmp_clone);
        git_config(&tmp_clone);
        write_file(&tmp_clone, "file.txt", "init\n");
        git_add_commit(&tmp_clone, "initial");
        git_push(&tmp_clone);

        // Clone working repo
        git_clone(&bare_origin, &work_repo);
        git_config(&work_repo);

        let _ = std::fs::remove_dir_all(&tmp_clone);

        // Locate grove binary
        let grove_bin = find_grove_bin();
        let worktree_base = temp_dir.path().join("grove_worktrees");

        TestRepo {
            temp_dir,
            work_repo,
            bare_origin,
            grove_bin,
            home_dir,
            worktree_base,
        }
    }

    pub fn work_repo(&self) -> &Path {
        &self.work_repo
    }

    pub fn run_grove(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(&self.grove_bin)
            .args(args)
            .current_dir(&self.work_repo)
            .env("HOME", &self.home_dir)
            .env("GROVE_WORKTREE_BASE", &self.worktree_base)
            .output()
            .expect("failed to run grove");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(1);

        (code, stdout, stderr)
    }

    pub fn create_branch(&self, name: &str) {
        let output = Command::new("git")
            .args(["branch", name])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "failed to create branch: {:?}",
            output
        );
    }

    pub fn checkout(&self, name: &str) {
        let output = Command::new("git")
            .args(["checkout", name])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    pub fn commit_on_branch(&self, branch: &str, name: &str, content: &str) {
        self.checkout(branch);
        write_file(&self.work_repo, name, content);
        git_add_commit(&self.work_repo, "wip");
    }

    pub fn merge_into_main(&self, branch: &str) {
        self.checkout("main");
        let output = Command::new("git")
            .args([
                "merge",
                "--no-ff",
                branch,
                "-m",
                &format!("merge {}", branch),
            ])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "failed to merge {} into main: {:?}",
            branch,
            output
        );
    }

    pub fn commit_and_push_remote_main(&self, name: &str, content: &str) {
        let remote_clone = self.temp_dir.path().join("remote_clone");
        git_clone(&self.bare_origin, &remote_clone);
        git_config(&remote_clone);
        write_file(&remote_clone, name, content);
        git_add_commit(&remote_clone, "remote update");
        git_push(&remote_clone);
        std::fs::remove_dir_all(remote_clone).unwrap();
    }

    pub fn head(&self) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    pub fn set_origin_url(&self, url: &Path) {
        let output = Command::new("git")
            .args(["remote", "set-url", "origin", &url.display().to_string()])
            .current_dir(&self.work_repo)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

fn find_grove_bin() -> PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let release = base.join("target").join("release").join("grove");
    let debug = base.join("target").join("debug").join("grove");

    if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        panic!("grove binary not found at {:?} or {:?}", release, debug);
    }
}

fn run_git_init_bare(path: &Path) {
    let output = Command::new("git")
        .args(["init", "--bare", &path.display().to_string(), "-b", "main"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn git_clone(source: &Path, dest: &Path) {
    let output = Command::new("git")
        .args([
            "clone",
            &source.display().to_string(),
            &dest.display().to_string(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn git_config(dir: &Path) {
    for (key, val) in &[("user.email", "test@test.com"), ("user.name", "Test")] {
        let output = Command::new("git")
            .args(["config", key, val])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

fn write_file(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn git_add_commit(dir: &Path, msg: &str) {
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "commit failed: {:?}", output);
}

fn git_push(dir: &Path) {
    let output = Command::new("git")
        .args(["push", "origin", "main"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success());
}
