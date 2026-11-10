use foxmorrow_core::Error;
use foxmorrow_core::journal::Journal;

const SEED: &str = "INSERT INTO run VALUES ('r', 1, '/p', 'task.md', 'h', 's', '[]', '[]', '[]', NULL, NULL, 1, '{}');
INSERT INTO assignment VALUES ('a-01', 1, 1, 'backend-worker', 'author', 'p', '[]', NULL, NULL, '[]', '[]', NULL);
INSERT INTO report VALUES ('a-01', 1, 'completed', 'passed', '{}', 'x');";

#[test]
fn creates_and_reopens_a_journal() {
    let dir = tempfile::tempdir().unwrap();
    Journal::create(dir.path()).unwrap();
    let journal = Journal::open(dir.path()).unwrap();
    assert!(dir.path().join("journal.sqlite").is_file());
    assert_eq!(journal.dir(), dir.path());
}

#[test]
fn refuses_missing_duplicate_and_foreign_journals() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(Journal::open(dir.path()), Err(Error::NotFound(_))));
    let journal = Journal::create(dir.path()).unwrap();
    assert!(matches!(
        Journal::create(dir.path()),
        Err(Error::Conflict(_))
    ));
    journal
        .conn()
        .execute(
            "UPDATE meta SET value = '999' WHERE key = 'contract_version'",
            [],
        )
        .unwrap();
    assert!(matches!(Journal::open(dir.path()), Err(Error::Invalid(_))));
}

#[test]
fn accepted_records_cannot_change() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::create(dir.path()).unwrap();
    journal.conn().execute_batch(SEED).unwrap();
    let error = journal
        .conn()
        .execute("UPDATE report SET outcome = 'failed'", [])
        .unwrap_err();
    assert!(error.to_string().contains("immutable: report"));
    let error = journal
        .conn()
        .execute("DELETE FROM assignment", [])
        .unwrap_err();
    assert!(error.to_string().contains("immutable: assignment"));
}

#[test]
fn final_closes_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::create(dir.path()).unwrap();
    journal.conn().execute_batch(SEED).unwrap();
    journal
        .conn()
        .execute(
            "INSERT INTO final VALUES ('r', 1, 'a-01', 'passed', '{}')",
            [],
        )
        .unwrap();
    let error = journal
        .conn()
        .execute(
            "INSERT INTO assignment VALUES ('a-02', 2, 1, 'reviewer', 'review', 'p', '[]', NULL, NULL, '[]', '[]', NULL)",
            [],
        )
        .unwrap_err();
    assert!(error.to_string().contains("run is finalized"));
}

#[test]
fn failed_write_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::create(dir.path()).unwrap();
    let result: foxmorrow_core::Result<()> = journal.write(|tx| {
        tx.execute("INSERT INTO meta (key, value) VALUES ('probe', 'x')", [])?;
        Err(Error::Invalid("stop".into()))
    });
    assert!(result.is_err());
    let count: i64 = journal
        .conn()
        .query_row("SELECT COUNT(*) FROM meta WHERE key = 'probe'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}
