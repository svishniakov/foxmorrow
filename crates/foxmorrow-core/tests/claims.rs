mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::claims::{self, ClaimState, ToolCall};
use foxmorrow_core::model::{Kind, VerificationState};
use foxmorrow_core::report;
use foxmorrow_core::state;

fn claim_state(claims: &[claims::Claim], id: &str) -> ClaimState {
    claims.iter().find(|claim| claim.id == id).unwrap().state
}

#[test]
fn matching_observation_is_accepted() {
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
    assert!(state::is_accepted(journal.conn(), &author.id).unwrap());
    assert_eq!(
        claims::accepted_observation(journal.conn(), &author.id)
            .unwrap()
            .unwrap()
            .agent_id,
        "agent-1"
    );
}

#[test]
fn missing_observation_stays_unverified_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    report::submit(&mut journal, &run, &author.id, &passed_report(&[])).unwrap();
    let first = claims::verify(&mut journal, &run, &author.id, None, &families()).unwrap();
    assert_eq!(first.state, VerificationState::Unverified);
    assert_eq!(claim_state(&first.claims, "C3"), ClaimState::Unverified);
    let second = claims::verify(
        &mut journal,
        &run,
        &author.id,
        Some(observation(&author, "agent-1", "swe-2-high")),
        &families(),
    )
    .unwrap();
    assert_eq!(second.state, VerificationState::Accepted);
    let third = claims::verify(
        &mut journal,
        &run,
        &author.id,
        Some(observation(&author, "agent-1", "swe-2-high")),
        &families(),
    );
    assert!(matches!(third, Err(Error::Conflict(_))));
}

#[test]
fn delivery_authorship_and_model_mismatches_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    report::submit(&mut journal, &run, &author.id, &passed_report(&[])).unwrap();
    let good = observation(&author, "agent-1", "swe-2-high");

    let cases: Vec<(&str, claims::ChainObservation)> = vec![
        (
            "C1",
            claims::ChainObservation {
                profile: "foxmorrow:reviewer".into(),
                ..good.clone()
            },
        ),
        (
            "C1",
            claims::ChainObservation {
                task_text: format!("assignment {}", author.id),
                ..good.clone()
            },
        ),
        (
            "C1",
            claims::ChainObservation {
                system_texts: vec!["plain role".into()],
                ..good.clone()
            },
        ),
        (
            "C2",
            claims::ChainObservation {
                tool_calls: Vec::new(),
                ..good.clone()
            },
        ),
        (
            "C3",
            claims::ChainObservation {
                models: vec!["gpt-6".into()],
                ..good.clone()
            },
        ),
    ];
    for (failed, observed) in cases {
        let verification =
            claims::verify(&mut journal, &run, &author.id, Some(observed), &families()).unwrap();
        assert_eq!(verification.state, VerificationState::Rejected, "{failed}");
        assert_eq!(
            claim_state(&verification.claims, failed),
            ClaimState::Failed,
            "{failed}"
        );
    }
}

#[test]
fn review_after_accepted_author_and_qa_after_passed_review() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let (_, _, qa) = accepted_chain(&mut journal, &run);
    assert_eq!(qa.id, "a-03");
}

#[test]
fn qa_requires_a_passed_review() {
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
        r#"{"summary":"defects","status":"completed","outcome":"failed"}"#,
    );
    let error = assignment::create(
        &mut journal,
        &run,
        spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id]),
    )
    .unwrap_err();
    assert!(error.to_string().contains("did not pass"), "{error}");
    let error = assignment::create(
        &mut journal,
        &run,
        spec("qa-verifier", Kind::Qa, "opus-5", &[&author.id]),
    )
    .unwrap_err();
    assert!(error.to_string().contains("passed review"), "{error}");
}

#[test]
fn qa_report_must_cover_every_criterion() {
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
    let partial = report::submit(&mut journal, &run, &qa.id, &passed_report(&["ac-1"]));
    assert!(partial.unwrap_err().to_string().contains("must cover ac-2"));
    let mixed = r#"{"summary":"x","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"r"},"ac-2":{"status":"failed","reason":"r"}}}"#;
    assert!(
        report::submit(&mut journal, &run, &qa.id, mixed)
            .unwrap_err()
            .to_string()
            .contains("every criterion")
    );
}

#[test]
fn family_rule_rejects_a_reviewer_of_the_author_family() {
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
    let review = assignment::create(
        &mut journal,
        &run,
        spec("reviewer", Kind::Review, "swe-2-low", &[&author.id]),
    )
    .unwrap();
    report::submit(&mut journal, &run, &review.id, &passed_report(&[])).unwrap();
    let verification = claims::verify(
        &mut journal,
        &run,
        &review.id,
        Some(observation(&review, "agent-2", "swe-2-low")),
        &families(),
    )
    .unwrap();
    assert_eq!(claim_state(&verification.claims, "C3"), ClaimState::Failed);

    let unknown = claims::verify(
        &mut journal,
        &run,
        &review.id,
        Some(observation(&review, "agent-2", "swe-2-low")),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(claim_state(&unknown.claims, "C3"), ClaimState::Unverified);
}

#[test]
fn family_rule_can_be_disabled_for_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run_with(dir.path(), false);
    let author = accept(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        "agent-1",
        "swe-2-high",
        &passed_report(&[]),
    );
    accept(
        &mut journal,
        &run,
        spec("reviewer", Kind::Review, "swe-2-low", &[&author.id]),
        "agent-2",
        "swe-2-low",
        &passed_report(&[]),
    );
}

#[test]
fn verification_chain_must_be_independent() {
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
    let review = assignment::create(
        &mut journal,
        &run,
        spec("reviewer", Kind::Review, "gpt-6", &[&author.id]),
    )
    .unwrap();
    report::submit(&mut journal, &run, &review.id, &passed_report(&[])).unwrap();

    let same_agent = claims::verify(
        &mut journal,
        &run,
        &review.id,
        Some(observation(&review, "agent-1", "gpt-6")),
        &families(),
    )
    .unwrap();
    assert_eq!(claim_state(&same_agent.claims, "C5"), ClaimState::Failed);

    let mut edited = observation(&review, "agent-2", "gpt-6");
    edited.tool_calls.push(ToolCall {
        id: "e1".into(),
        name: "edit".into(),
        command: None,
        exit_code: None,
    });
    let editing =
        claims::verify(&mut journal, &run, &review.id, Some(edited), &families()).unwrap();
    assert_eq!(claim_state(&editing.claims, "C5"), ClaimState::Failed);
}

#[test]
fn tool_call_evidence_must_be_in_the_chain_and_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    report::submit(
        &mut journal,
        &run,
        &author.id,
        r#"{"summary":"x","status":"completed","outcome":"passed","criteria":{"ac-1":{"status":"passed","reason":"r","evidence":["exec-7"]}}}"#,
    )
    .unwrap();
    let missing = claims::verify(
        &mut journal,
        &run,
        &author.id,
        Some(observation(&author, "agent-1", "swe-2-high")),
        &families(),
    )
    .unwrap();
    assert_eq!(claim_state(&missing.claims, "C4"), ClaimState::Failed);

    let mut failing = observation(&author, "agent-1", "swe-2-high");
    failing.tool_calls.push(ToolCall {
        id: "exec-7".into(),
        name: "exec".into(),
        command: Some("cargo test".into()),
        exit_code: Some(101),
    });
    let failed =
        claims::verify(&mut journal, &run, &author.id, Some(failing), &families()).unwrap();
    assert_eq!(claim_state(&failed.claims, "C4"), ClaimState::Failed);
}

#[cfg(unix)]
#[test]
fn check_evidence_must_be_run_from_the_chain() {
    use foxmorrow_core::receipt;

    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let author = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    let check = receipt::run_check(
        &mut journal,
        &author.id,
        dir.path(),
        &["sh".into(), "-c".into(), "echo ok".into()],
    )
    .unwrap();
    report::submit(
        &mut journal,
        &run,
        &author.id,
        &format!(
            r#"{{"summary":"x","status":"completed","outcome":"passed","evidence":["{}"]}}"#,
            check.id
        ),
    )
    .unwrap();

    let without = claims::verify(
        &mut journal,
        &run,
        &author.id,
        Some(observation(&author, "agent-1", "swe-2-high")),
        &families(),
    )
    .unwrap();
    assert_eq!(claim_state(&without.claims, "C4"), ClaimState::Failed);

    let mut with = observation(&author, "agent-1", "swe-2-high");
    with.tool_calls.push(ToolCall {
        id: "exec-1".into(),
        name: "exec".into(),
        command: Some(format!(
            "bin/foxmorrow check -a {} -- sh -c \"echo ok\"",
            author.id
        )),
        exit_code: Some(0),
    });
    let verified = claims::verify(&mut journal, &run, &author.id, Some(with), &families()).unwrap();
    assert_eq!(
        verified.state,
        VerificationState::Accepted,
        "{:?}",
        verified.claims
    );
}
