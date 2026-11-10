#![cfg(windows)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn downloads_verifies_and_runs_on_windows() {
    let plugin = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(plugin.path().join("bin")).unwrap();
    for name in ["foxmorrow.cmd", "foxmorrow.ps1"] {
        std::fs::copy(
            repo().join("bin").join(name),
            plugin.path().join("bin").join(name),
        )
        .unwrap();
    }
    let release = assets.path().join("v9.9.9");
    std::fs::create_dir_all(&release).unwrap();
    let asset = release.join("foxmorrow-x86_64-pc-windows-msvc.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_foxmorrow"), &asset).unwrap();
    let sha = foxmorrow_core::util::sha256_hex(&std::fs::read(&asset).unwrap());
    let write = |sha: &str| {
        std::fs::write(
            plugin.path().join("checksums.json"),
            format!("{{\n  \"version\": \"9.9.9\",\n  \"repository\": \"test/foxmorrow\",\n  \"binaries\": {{\n    \"x86_64-pc-windows-msvc\": \"{sha}\"\n  }}\n}}\n"),
        )
        .unwrap();
    };
    let base = format!(
        "file:///{}",
        assets.path().display().to_string().replace('\\', "/")
    );
    let run = || {
        Command::new(plugin.path().join("bin/foxmorrow.cmd"))
            .arg("--version")
            .env("FOXMORROW_RELEASE_BASE", &base)
            .env("LOCALAPPDATA", cache.path())
            .env_remove("FOXMORROW_DEV_BINARY")
            .output()
            .unwrap()
    };

    write(&"0".repeat(64));
    let refused = run();
    assert_eq!(
        refused.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(String::from_utf8_lossy(&refused.stderr).contains("checksum mismatch"));

    write(&sha);
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("foxmorrow "));
    assert!(
        cache
            .path()
            .join("foxmorrow/9.9.9/x86_64-pc-windows-msvc/foxmorrow.exe")
            .is_file()
    );
}
