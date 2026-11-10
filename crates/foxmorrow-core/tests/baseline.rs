use std::path::Path;
use std::process::Command;

use foxmorrow_core::baseline;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn records_head_and_dirty_files_without_foxmorrow_data() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    std::fs::write(dir.path().join("a.txt"), "one").unwrap();
    git(dir.path(), &["add", "a.txt"]);
    git(dir.path(), &["commit", "-qm", "init"]);
    std::fs::write(dir.path().join("a.txt"), "two").unwrap();
    std::fs::write(dir.path().join("b.txt"), "new").unwrap();
    std::fs::create_dir_all(dir.path().join(".foxmorrow")).unwrap();
    std::fs::write(dir.path().join(".foxmorrow/current"), "r-1").unwrap();

    let state = baseline::collect(dir.path()).unwrap();

    assert_eq!(state.head.as_deref().map(str::len), Some(40));
    assert!(
        state
            .dirty
            .iter()
            .any(|line| line.ends_with("a.txt") && line.starts_with(" M"))
    );
    assert!(state.dirty.iter().any(|line| line == "?? b.txt"));
    assert!(!state.dirty.iter().any(|line| line.contains(".foxmorrow")));
    assert!(state.remote_refs.is_empty());
}

#[test]
fn empty_repository_has_no_head() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    assert_eq!(baseline::collect(dir.path()).unwrap().head, None);
}

#[test]
fn directory_outside_git_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(baseline::collect(dir.path()).is_err());
}
