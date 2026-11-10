#![cfg(unix)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn foxmorrow(dir: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_foxmorrow"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    if let Some(input) = stdin {
        pipe.write_all(input.as_bytes()).unwrap();
    }
    drop(pipe);
    child.wait_with_output().unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        dir.path().join("task.md"),
        "# Task\n\n## Критерии приёмки\n\n- Health returns 200\n",
    )
    .unwrap();
    dir
}

const ASSIGN: &[&str] = &[
    "assign",
    "--role",
    "backend-worker",
    "--kind",
    "author",
    "--profile",
    "foxmorrow:backend-worker",
    "--expected-model",
    "swe-2-high",
    "--paths",
    "src/",
];

#[test]
fn runs_init_assign_check_report_status() {
    let dir = project();
    let root = dir.path();
    let init = json(&foxmorrow(
        root,
        &[
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--paths",
            "src/",
        ],
        None,
    ));
    assert_eq!(init["criteria"][0]["id"], "ac-1");

    let assigned = json(&foxmorrow(root, ASSIGN, None));
    assert_eq!(assigned["id"], "a-01");

    let check = foxmorrow(
        root,
        &[
            "check",
            "-a",
            "a-01",
            "--",
            "sh",
            "-c",
            "echo hello; exit 3",
        ],
        None,
    );
    assert_eq!(check.status.code(), Some(3));
    assert_eq!(String::from_utf8_lossy(&check.stdout), "hello\n");
    assert!(String::from_utf8_lossy(&check.stderr).contains("chk-001 exit=3"));

    let reported = json(&foxmorrow(
        root,
        &["report", "-a", "a-01"],
        Some(
            r#"{"summary":"tried","status":"completed","outcome":"failed","evidence":["chk-001"]}"#,
        ),
    ));
    assert_eq!(reported["outcome"], "failed");

    let status = json(&foxmorrow(root, &["status"], None));
    assert_eq!(status["assignments"][0]["report"]["outcome"], "failed");
    assert_eq!(
        status["assignments"][0]["verification"],
        serde_json::Value::Null
    );
}

#[test]
fn errors_exit_with_code_two() {
    let dir = project();
    let root = dir.path();
    json(&foxmorrow(
        root,
        &[
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--paths",
            "src/",
        ],
        None,
    ));
    json(&foxmorrow(root, ASSIGN, None));
    let report = r#"{"summary":"x","status":"completed","outcome":"failed"}"#;
    json(&foxmorrow(root, &["report", "-a", "a-01"], Some(report)));
    let again = foxmorrow(root, &["report", "-a", "a-01"], Some(report));
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("already has a report"));
}

#[test]
fn status_outside_a_run_fails() {
    let dir = tempfile::tempdir().unwrap();
    let output = foxmorrow(dir.path(), &["status"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no foxmorrow run"));
}
