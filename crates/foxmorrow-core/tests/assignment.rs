use foxmorrow_core::Error;
use foxmorrow_core::assignment::{self, NewAssignment, within};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};

fn setup(dir: &std::path::Path) -> (foxmorrow_core::run::Run, foxmorrow_core::journal::Journal) {
    run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: "## Критерии приёмки\n\n- Первый\n".into(),
        allowed_paths: vec!["src/".into(), "tests/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule: true,
    })
    .unwrap()
}

fn author(paths: &[&str]) -> NewAssignment {
    NewAssignment {
        role: "backend-worker".into(),
        kind: Kind::Author,
        profile: Some("foxmorrow:backend-worker".into()),
        expected_models: vec!["swe-2-high".into()],
        brief_sha256: Some("brief".into()),
        role_sha256: Some("role".into()),
        allowed_paths: paths.iter().map(|p| p.to_string()).collect(),
        depends_on: Vec::new(),
        previous_id: None,
    }
}

#[test]
fn numbers_assignments_and_reads_them_back() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let first = assignment::create(&mut journal, &run, author(&["src/api"])).unwrap();
    let second = assignment::create(&mut journal, &run, author(&["./tests/"])).unwrap();
    assert_eq!((first.id.as_str(), second.id.as_str()), ("a-01", "a-02"));
    assert_eq!(assignment::get(journal.conn(), "a-01").unwrap(), first);
    assert_eq!(
        assignment::list(journal.conn()).unwrap(),
        vec![first, second]
    );
}

#[test]
fn keeps_paths_inside_the_run() {
    assert!(within("src/api/health.rs", &["src/".into()]));
    assert!(within("src", &["./src/".into()]));
    assert!(!within("srcx/file", &["src".into()]));
    assert!(!within("src/../secrets", &["src".into()]));
    assert!(within("anything", &[".".into()]));

    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    assert!(matches!(
        assignment::create(&mut journal, &run, author(&["docs/"])),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        assignment::create(&mut journal, &run, author(&[])),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn delegated_assignments_need_profile_and_model() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let mut missing_profile = author(&["src/"]);
    missing_profile.profile = None;
    assert!(matches!(
        assignment::create(&mut journal, &run, missing_profile),
        Err(Error::Invalid(_))
    ));
    let mut missing_model = author(&["src/"]);
    missing_model.expected_models.clear();
    assert!(matches!(
        assignment::create(&mut journal, &run, missing_model),
        Err(Error::Invalid(_))
    ));
    let mut root = author(&["src/"]);
    root.kind = Kind::Root;
    root.profile = None;
    root.expected_models.clear();
    assert!(assignment::create(&mut journal, &run, root).is_ok());
}

#[test]
fn enforces_dependency_links() {
    let dir = tempfile::tempdir().unwrap();
    let (run, mut journal) = setup(dir.path());
    let mut unknown = author(&["src/"]);
    unknown.depends_on = vec!["a-99".into()];
    assert!(matches!(
        assignment::create(&mut journal, &run, unknown),
        Err(Error::NotFound(_))
    ));

    let written = assignment::create(&mut journal, &run, author(&["src/"])).unwrap();
    let mut review = author(&["src/"]);
    review.kind = Kind::Review;
    assert!(matches!(
        assignment::create(&mut journal, &run, review.clone()),
        Err(Error::Invalid(_))
    ));
    review.depends_on = vec![written.id.clone()];
    let error = assignment::create(&mut journal, &run, review).unwrap_err();
    assert!(error.to_string().contains("has no report"), "{error}");

    let mut correction = author(&["src/"]);
    correction.previous_id = Some(written.id.clone());
    let error = assignment::create(&mut journal, &run, correction).unwrap_err();
    assert!(error.to_string().contains("has no report"), "{error}");
}
