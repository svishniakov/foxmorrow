#![cfg(unix)]

use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::model::Kind;
use foxmorrow_core::receipt::{self, OUTPUT_TAIL_BYTES};
use foxmorrow_core::run::{self, NewRun};

fn setup(dir: &std::path::Path) -> (foxmorrow_core::run::Run, foxmorrow_core::journal::Journal) {
    let (run, mut journal) = run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n".into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap();
    assignment::create(
        &mut journal,
        &run,
        NewAssignment {
            role: "backend-worker".into(),
            kind: Kind::Author,
            profile: Some("foxmorrow:backend-worker".into()),
            expected_models: vec!["swe-2-high".into()],
            brief_sha256: None,
            role_sha256: None,
            allowed_paths: vec!["src/".into()],
            depends_on: Vec::new(),
            previous_id: None,
        },
    )
    .unwrap();
    (run, journal)
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

#[test]
fn records_success_and_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let ok = receipt::run_check(
        &mut journal,
        "a-01",
        dir.path(),
        &argv(&["sh", "-c", "echo ok"]),
    )
    .unwrap();
    assert_eq!(ok.id, "chk-001");
    assert_eq!(ok.exit_code, Some(0));
    assert_eq!(ok.output_tail.as_deref(), Some("ok\n"));
    assert!(ok.finished_ms.is_some());

    let failed = receipt::run_check(
        &mut journal,
        "a-01",
        dir.path(),
        &argv(&["sh", "-c", "echo bad >&2; exit 3"]),
    )
    .unwrap();
    assert_eq!(failed.id, "chk-002");
    assert_eq!(failed.exit_code, Some(3));
    assert_eq!(failed.output_tail.as_deref(), Some("bad\n"));
    assert_eq!(
        receipt::list_for(journal.conn(), "a-01").unwrap(),
        vec![ok, failed]
    );
}

#[test]
fn records_a_command_that_cannot_start() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let missing = receipt::run_check(
        &mut journal,
        "a-01",
        dir.path(),
        &argv(&["foxmorrow-no-such-binary"]),
    )
    .unwrap();
    assert_eq!(missing.exit_code, None);
    assert!(missing.output_tail.unwrap().contains("failed to start"));
}

#[test]
fn keeps_only_the_output_tail() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    let script = format!(
        "head -c {} /dev/zero | tr '\\0' 'x'",
        OUTPUT_TAIL_BYTES + 100
    );
    let long = receipt::run_check(
        &mut journal,
        "a-01",
        dir.path(),
        &argv(&["sh", "-c", &script]),
    )
    .unwrap();
    assert_eq!(long.output_tail.unwrap().len(), OUTPUT_TAIL_BYTES);
    assert_eq!(long.output_sha256.unwrap().len(), 64);
}

#[test]
fn finished_receipt_cannot_change() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = setup(dir.path());
    receipt::run_check(&mut journal, "a-01", dir.path(), &argv(&["true"])).unwrap();
    let error = journal
        .conn()
        .execute("UPDATE receipt SET exit_code = 1", [])
        .unwrap_err();
    assert!(error.to_string().contains("immutable: receipt"));
}
