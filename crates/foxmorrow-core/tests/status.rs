mod common;

use common::*;
use foxmorrow_core::assignment;
use foxmorrow_core::finalize;
use foxmorrow_core::model::{Kind, Outcome, VerificationState};
use foxmorrow_core::status;

#[test]
fn summarizes_assignments_reports_and_final() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = new_run(dir.path());
    let fresh = status::status(journal.conn(), &run).unwrap();
    assert!(fresh.assignments.is_empty());
    assert_eq!(fresh.criteria.len(), 2);

    let (author, _, qa) = accepted_chain(&mut journal, &run);
    let open = assignment::create(
        &mut journal,
        &run,
        spec("python-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    journal
        .conn()
        .execute(
            "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES ('chk-900', 900, ?1, '/', '[\"make\"]', 1)",
            [&open.id],
        )
        .unwrap();

    let view = status::status(journal.conn(), &run).unwrap();
    let first = view
        .assignments
        .iter()
        .find(|entry| entry.id == author.id)
        .unwrap();
    assert_eq!(first.verification, Some(VerificationState::Accepted));
    assert_eq!(first.report.as_ref().unwrap().outcome, Outcome::Passed);
    let pending = view
        .assignments
        .iter()
        .find(|entry| entry.id == open.id)
        .unwrap();
    assert_eq!(pending.open_checks, vec!["chk-900".to_string()]);
    assert!(pending.report.is_none());
    assert_eq!(view.final_outcome, None);

    finalize::finalize(&mut journal, &run, &qa.id, &families()).unwrap();
    assert_eq!(
        status::status(journal.conn(), &run).unwrap().final_outcome,
        Some(Outcome::Passed)
    );
}
