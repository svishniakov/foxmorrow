use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::journal::Journal;
use foxmorrow_core::model::{Kind, Outcome};
use foxmorrow_core::report;
use foxmorrow_core::run::{self, NewRun, Run};

fn setup(dir: &std::path::Path, kind: Kind) -> (Run, Journal) {
    let (run, mut journal) = run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n- Второй\n".into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap();
    let delegated = kind != Kind::Root;
    assignment::create(
        &mut journal,
        &run,
        NewAssignment {
            role: if delegated {
                "backend-worker".into()
            } else {
                "root".into()
            },
            kind,
            profile: delegated.then(|| "foxmorrow:backend-worker".to_string()),
            expected_models: if delegated {
                vec!["swe-2-high".into()]
            } else {
                Vec::new()
            },
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

fn invalid(result: foxmorrow_core::Result<report::ReportBody>, needle: &str) {
    match result {
        Err(Error::Invalid(message)) => assert!(message.contains(needle), "{message}"),
        other => panic!("expected invalid '{needle}', got {other:?}"),
    }
}

#[test]
fn stores_one_report_per_assignment() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Author);
    let body = report::submit(
        &mut journal,
        &run,
        "a-01",
        r#"{"summary":"done","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"tested"}}}"#,
    )
    .unwrap();
    let (stored, sha) = report::get(journal.conn(), "a-01").unwrap().unwrap();
    assert_eq!(stored, body);
    assert_eq!(sha.len(), 64);
    let again = report::submit(
        &mut journal,
        &run,
        "a-01",
        r#"{"summary":"x","status":"completed","outcome":"failed"}"#,
    );
    assert!(matches!(again, Err(Error::Conflict(_))));
}

#[test]
fn rejects_malformed_reports() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Author);
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":" ","status":"completed","outcome":"failed"}"#,
        ),
        "summary",
    );
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"x","status":"interrupted","outcome":"passed"}"#,
        ),
        "completed",
    );
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"x","status":"completed","outcome":"failed","criteria":{"ac-9":{"status":"failed","reason":"r"}}}"#,
        ),
        "unknown criterion",
    );
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"x","status":"completed","outcome":"failed","criteria":{"ac-1":{"status":"failed","reason":""}}}"#,
        ),
        "reason",
    );
    assert!(matches!(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"x","status":"completed","outcome":"passed","extra":1}"#
        ),
        Err(Error::Json(_))
    ));
    assert!(report::get(journal.conn(), "a-01").unwrap().is_none());
}

#[test]
fn root_stop_needs_reason_and_unverified_list() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path(), Kind::Root);
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"stop","status":"completed","outcome":"failed"}"#,
        ),
        "reason",
    );
    invalid(
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"stop","status":"completed","outcome":"unverified","reason":"no access"}"#,
        ),
        "unverified list",
    );
    let body = report::submit(
        &mut journal,
        &run,
        "a-01",
        r#"{"summary":"stop","status":"completed","outcome":"unverified","reason":"no access","unverified":["ac-2"]}"#,
    )
    .unwrap();
    assert_eq!(body.outcome, Outcome::Unverified);
}

#[cfg(unix)]
mod with_checks {
    use super::*;
    use foxmorrow_core::receipt;

    fn check(journal: &mut Journal, dir: &std::path::Path, script: &str) -> String {
        receipt::run_check(
            journal,
            "a-01",
            dir,
            &["sh".into(), "-c".into(), script.into()],
        )
        .unwrap()
        .id
    }

    #[test]
    fn passing_evidence_must_have_exit_code_zero() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        let failed = check(&mut journal, dir.path(), "exit 1");
        let body = format!(
            r#"{{"summary":"x","status":"completed","outcome":"failed","criteria":{{"ac-1":{{"status":"passed","reason":"r","evidence":["{failed}"]}}}}}}"#
        );
        invalid(
            report::submit(&mut journal, &run, "a-01", &body),
            "did not succeed",
        );
        let honest = format!(
            r#"{{"summary":"x","status":"completed","outcome":"failed","evidence":["{failed}"]}}"#
        );
        assert!(report::submit(&mut journal, &run, "a-01", &honest).is_ok());
    }

    #[test]
    fn evidence_must_belong_to_the_assignment() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        invalid(
            report::submit(
                &mut journal,
                &run,
                "a-01",
                r#"{"summary":"x","status":"completed","outcome":"failed","evidence":["chk-404"]}"#,
            ),
            "does not belong",
        );
    }

    #[test]
    fn check_without_result_forces_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        journal
            .conn()
            .execute(
                "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES ('chk-001', 1, 'a-01', '/', '[\"make\"]', 1)",
                [],
            )
            .unwrap();
        invalid(
            report::submit(
                &mut journal,
                &run,
                "a-01",
                r#"{"summary":"x","status":"completed","outcome":"failed"}"#,
            ),
            "interrupted",
        );
        assert!(
            report::submit(
                &mut journal,
                &run,
                "a-01",
                r#"{"summary":"x","status":"interrupted","outcome":"unverified"}"#
            )
            .is_ok()
        );
    }

    #[test]
    fn no_checks_after_the_report() {
        let dir = tempfile::tempdir().unwrap();
        let (run, mut journal) = setup(dir.path(), Kind::Author);
        report::submit(
            &mut journal,
            &run,
            "a-01",
            r#"{"summary":"x","status":"completed","outcome":"failed"}"#,
        )
        .unwrap();
        let late = receipt::run_check(&mut journal, "a-01", dir.path(), &["true".into()]);
        assert!(matches!(late, Err(Error::Conflict(_))));
    }
}
