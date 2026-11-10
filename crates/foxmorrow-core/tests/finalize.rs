mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::claims;
use foxmorrow_core::finalize;
use foxmorrow_core::model::{Kind, Outcome, VerificationState};
use foxmorrow_core::report;

#[test]
fn closes_an_accepted_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (author, review, qa) = accepted_chain(&mut journal, &run);
    let record = finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap();
    assert_eq!(record.outcome, Outcome::Passed);
    assert_eq!(
        record.reports,
        vec![author.id.clone(), review.id, qa.id.clone()]
    );
    assert!(record.family_rule);

    assert!(matches!(
        finalize::finalize(&mut journal, &run, &qa.id, &families()),
        Err(Error::Conflict(_))
    ));
    let late = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    );
    assert!(late.unwrap_err().to_string().contains("run is finalized"));
}

#[test]
fn only_qa_or_root_reports_close_a_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (author, _, _) = accepted_chain(&mut journal, &run);
    assert!(matches!(
        finalize::finalize(&mut journal, &run, &author.id, &families()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn unaccepted_qa_cannot_close_a_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = accept(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        "agent-1",
        "swe-2-high",
        &passed_report(&[]),
    );
    let review = accept(
        &mut journal,
        &run,
        spec("reviewer", Kind::Review, "gpt-6", &[&author.id]),
        "agent-2",
        "gpt-6",
        &passed_report(&[]),
    );
    let qa = assignment::create(
        &mut journal,
        &run,
        spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id]),
    )
    .unwrap();
    report::submit(
        &mut journal,
        &run,
        &qa.id,
        &passed_report(&["ac-1", "ac-2"]),
    )
    .unwrap();
    let error = finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap_err();
    assert!(error.to_string().contains("not accepted"), "{error}");
}

#[test]
fn snapshots_are_rechecked() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (_, _, qa) = accepted_chain(&mut journal, &run);
    let mut merged = families();
    merged.insert("gpt-6".into(), "swe".into());
    let error = finalize::finalize(&mut journal, &run, &qa.id, &merged).unwrap_err();
    assert!(error.to_string().contains("no longer verifies"), "{error}");
}

#[test]
fn root_can_stop_with_a_non_positive_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let root = assignment::create(&mut journal, &run, root_spec()).unwrap();
    report::submit(
        &mut journal,
        &run,
        &root.id,
        r#"{"summary":"stopped","status":"completed","outcome":"unverified","reason":"no browser access","unverified":["ac-2"]}"#,
    )
    .unwrap();
    let mut observed = observation(&root, "root", "opus-5");
    observed.profile = String::new();
    let verification =
        claims::verify(&mut journal, &run, &root.id, Some(observed), &families()).unwrap();
    assert_eq!(
        verification.state,
        VerificationState::Accepted,
        "{:?}",
        verification.claims
    );
    let record = finalize::finalize(&mut journal, &run, &root.id, &families()).unwrap();
    assert_eq!(record.outcome, Outcome::Unverified);
    assert_eq!(record.reason.as_deref(), Some("no browser access"));
    assert_eq!(record.unverified, vec!["ac-2".to_string()]);
}

#[test]
fn root_cannot_claim_a_passed_final() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let root = assignment::create(&mut journal, &run, root_spec()).unwrap();
    report::submit(&mut journal, &run, &root.id, &passed_report(&[])).unwrap();
    let mut observed = observation(&root, "root", "opus-5");
    observed.profile = String::new();
    claims::verify(&mut journal, &run, &root.id, Some(observed), &families()).unwrap();
    assert!(matches!(
        finalize::finalize(&mut journal, &run, &root.id, &families()),
        Err(Error::Invalid(_))
    ));
}
