#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FAKE: &str = "#!/bin/sh\necho \"fake $* root=$FOXMORROW_PLUGIN_ROOT\"\n";

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        other => panic!("unsupported test platform {other:?}"),
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn executable(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn checksums(dir: &Path) -> String {
    let output = Command::new("sh")
        .arg(repo().join("scripts/write-checksums.sh"))
        .args(["9.9.9", "test/foxmorrow"])
        .arg(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

struct Release {
    plugin: tempfile::TempDir,
    assets: tempfile::TempDir,
    cache: tempfile::TempDir,
}

impl Release {
    fn new() -> Self {
        let release = Release {
            plugin: tempfile::tempdir().unwrap(),
            assets: tempfile::tempdir().unwrap(),
            cache: tempfile::tempdir().unwrap(),
        };
        let loader = std::fs::read_to_string(repo().join("bin/foxmorrow")).unwrap();
        executable(&release.plugin.path().join("bin/foxmorrow"), &loader);
        executable(&release.asset(), FAKE);
        let json = checksums(&release.assets.path().join("v9.9.9"));
        std::fs::write(release.plugin.path().join("checksums.json"), json).unwrap();
        release
    }

    fn asset(&self) -> PathBuf {
        self.assets
            .path()
            .join("v9.9.9")
            .join(format!("foxmorrow-{}", target()))
    }

    fn cached(&self) -> PathBuf {
        self.cache
            .path()
            .join("foxmorrow/9.9.9")
            .join(target())
            .join("foxmorrow")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(self.plugin.path().join("bin/foxmorrow"))
            .args(args)
            .env(
                "FOXMORROW_RELEASE_BASE",
                format!("file://{}", self.assets.path().display()),
            )
            .env("XDG_CACHE_HOME", self.cache.path())
            .env_remove("FOXMORROW_DEV_BINARY")
            .output()
            .unwrap()
    }
}

fn failed(output: &Output, needle: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn writes_checksums_for_present_binaries() {
    let empty = tempfile::tempdir().unwrap();
    let value: serde_json::Value = serde_json::from_str(&checksums(empty.path())).unwrap();
    assert_eq!(value["version"], "9.9.9");
    assert_eq!(value["repository"], "test/foxmorrow");
    assert!(value["binaries"].as_object().unwrap().is_empty());

    let release = Release::new();
    let json = std::fs::read_to_string(release.plugin.path().join("checksums.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let sha = value["binaries"][target()].as_str().unwrap();
    assert_eq!(sha, foxmorrow_core::util::sha256_hex(FAKE.as_bytes()));
}

#[test]
fn downloads_verifies_caches_and_runs() {
    let release = Release::new();
    let output = release.run(&["status", "--run", "r-1"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with("fake status --run r-1 root="),
        "{stdout}"
    );
    let plugin_name = release.plugin.path().file_name().unwrap().to_str().unwrap();
    assert!(stdout.trim_end().ends_with(plugin_name), "{stdout}");
    assert!(release.cached().is_file());

    std::fs::remove_file(release.asset()).unwrap();
    assert!(release.run(&["again"]).status.success());
}

#[test]
fn refuses_a_wrong_checksum_and_leaves_no_cache() {
    let release = Release::new();
    let path = release.plugin.path().join("checksums.json");
    let json = std::fs::read_to_string(&path).unwrap();
    let sha = foxmorrow_core::util::sha256_hex(FAKE.as_bytes());
    std::fs::write(&path, json.replace(&sha, &"0".repeat(64))).unwrap();
    failed(&release.run(&["status"]), "checksum mismatch");
    let dir = release.cached().parent().unwrap().to_path_buf();
    assert!(
        std::fs::read_dir(&dir)
            .map(|entries| entries.count())
            .unwrap_or(0)
            == 0
    );
}

#[test]
fn reports_a_missing_release_or_download_failure() {
    let release = Release::new();
    std::fs::remove_file(release.asset()).unwrap();
    failed(&release.run(&["status"]), "download failed");

    let empty = tempfile::tempdir().unwrap();
    std::fs::write(
        release.plugin.path().join("checksums.json"),
        checksums(empty.path()),
    )
    .unwrap();
    failed(&release.run(&["status"]), "no released binary");
}

#[test]
fn dev_binary_skips_the_download() {
    let release = Release::new();
    let dev = release.plugin.path().join("dev-foxmorrow");
    executable(&dev, "#!/bin/sh\necho \"dev $*\"\n");
    let output = Command::new(release.plugin.path().join("bin/foxmorrow"))
        .arg("doctor")
        .env("FOXMORROW_DEV_BINARY", &dev)
        .env("FOXMORROW_RELEASE_BASE", "file:///nonexistent")
        .env("XDG_CACHE_HOME", release.cache.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "dev doctor\n");
    assert!(!release.cached().exists());
}
