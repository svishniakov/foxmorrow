use foxmorrow_core::Error;
use foxmorrow_core::run::{self, NewRun};

fn new_run(root: &std::path::Path, paths: Vec<String>) -> NewRun {
    NewRun {
        project_root: root.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n- Второй\n".into(),
        allowed_paths: paths,
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
        skills_summary: Default::default(),
    }
}

#[test]
fn creates_a_run_and_points_current_at_it() {
    let dir = tempfile::tempdir().unwrap();
    let (created, _) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();

    assert!(
        dir.path()
            .join(".foxmorrow/runs")
            .join(&created.id)
            .join("journal.sqlite")
            .is_file()
    );
    assert_eq!(run::current_run_id(dir.path()).unwrap(), created.id);
    assert_eq!(created.criteria.len(), 2);
    assert_eq!(created.criteria_sha256.len(), 64);

    let (opened, _) = run::open_run(dir.path(), None).unwrap();
    assert_eq!(opened, created);
}

#[test]
fn requires_allowed_paths_and_an_existing_run() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        run::create_run(new_run(dir.path(), Vec::new())),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        run::open_run(dir.path(), None),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn root_identity_is_set_once_and_criteria_never_change() {
    let dir = tempfile::tempdir().unwrap();
    let (_, journal) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();
    journal
        .conn()
        .execute("UPDATE run SET root_model = 'opus'", [])
        .unwrap();
    assert!(
        journal
            .conn()
            .execute("UPDATE run SET root_model = 'gpt'", [])
            .is_err()
    );
    assert!(
        journal
            .conn()
            .execute("UPDATE run SET criteria_json = '[]'", [])
            .is_err()
    );
}

#[test]
fn records_root_identity_once() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut journal) = run::create_run(new_run(dir.path(), vec!["src/".into()])).unwrap();
    let first = run::record_root(&mut journal, Some("s-1"), Some("claude-opus-5-5-high")).unwrap();
    assert_eq!(first.root_session_id.as_deref(), Some("s-1"));
    assert_eq!(first.root_model.as_deref(), Some("claude-opus-5-5-high"));
    let second = run::record_root(&mut journal, Some("s-2"), Some("other")).unwrap();
    assert_eq!(second.root_session_id.as_deref(), Some("s-1"));
    assert_eq!(second.root_model.as_deref(), Some("claude-opus-5-5-high"));
    assert_eq!(run::record_root(&mut journal, None, None).unwrap(), second);
}
