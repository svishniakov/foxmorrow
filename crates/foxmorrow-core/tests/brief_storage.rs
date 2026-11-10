mod common;

use std::collections::BTreeMap;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, BriefDraft};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::util::sha256_hex;

#[test]
fn stores_the_rendered_brief_with_its_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (created, text) = assignment::create_with_brief(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        |draft| {
            Ok(BriefDraft {
                language: "en".into(),
                body: format!("Role: {}\n", draft.role),
            })
        },
    )
    .unwrap();
    let digest = created.brief_sha256.clone().unwrap();
    assert_eq!(digest, sha256_hex(b"Role: backend-worker\n"));
    assert_eq!(
        text,
        format!(
            "foxmorrow assignment a-01 run {} brief-sha256 {digest}\n\nRole: backend-worker\n",
            run.id
        )
    );
    assert_eq!(
        assignment::brief(journal.conn(), "a-01").unwrap(),
        Some(("en".to_string(), text))
    );
    assert_eq!(
        assignment::get(journal.conn(), "a-01")
            .unwrap()
            .brief_sha256,
        Some(digest)
    );
    assert!(
        journal
            .conn()
            .execute("UPDATE brief SET text = 'x'", [])
            .is_err()
    );
}

#[test]
fn failed_rendering_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let result = assignment::create_with_brief(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        |_| Err(Error::Invalid("template broken".into())),
    );
    assert!(matches!(result, Err(Error::Invalid(_))));
    assert!(assignment::list(journal.conn()).unwrap().is_empty());
}

#[test]
fn run_keeps_the_skills_summary() {
    let dir = tempfile::tempdir().unwrap();
    let summary = BTreeMap::from([(
        "architect".to_string(),
        vec!["arch-simplicity-guard".to_string()],
    )]);
    let (created, journal) = run::create_run(NewRun {
        project_root: dir.path().to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: TASK.into(),
        allowed_paths: vec!["src/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
        skills_summary: summary.clone(),
    })
    .unwrap();
    assert_eq!(created.skills_summary, summary);
    assert_eq!(
        run::load_run(journal.conn()).unwrap().skills_summary,
        summary
    );
    assert!(
        journal
            .conn()
            .execute("UPDATE run SET skills_summary_json = '{}'", [])
            .is_err()
    );
}
