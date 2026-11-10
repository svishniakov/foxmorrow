use std::path::Path;

use foxmorrow_core::Error;
use foxmorrow_core::document;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn doc(status: &str, based_on: &str, body: &str) -> String {
    format!("# Title\n\nDocument status: {status}\nBased on: {based_on}\n\n## Body\n\n{body}\n")
}

#[test]
fn reads_the_header() {
    let parsed = document::parse(
        Path::new("/p/docs/plan/impl.md"),
        &doc(
            "Approved",
            "[design](../design/d.md), [section](../prd/p.md#scope), [site](https://example.com), [here](#top)",
            "Document status: draft\nBased on: [late](late.md)",
        ),
    );
    assert_eq!(parsed.status.as_deref(), Some("approved"));
    assert_eq!(
        parsed.based_on,
        vec![
            Path::new("/p/docs/plan/../design/d.md").to_path_buf(),
            Path::new("/p/docs/plan/../prd/p.md").to_path_buf()
        ]
    );
    assert!(!parsed.open_questions);
    assert_eq!(
        document::parse(Path::new("x.md"), "# Only a title\n").status,
        None
    );
}

#[test]
fn finds_open_questions() {
    let open =
        |body: &str| document::parse(Path::new("x.md"), &doc("draft", "", body)).open_questions;
    assert!(open(
        "## 11. Открытые вопросы\n\n1. Кто владелец репозитория?"
    ));
    assert!(open("### Open questions\n\n- Which region?"));
    assert!(!open("## Открытые вопросы\n\nНет."));
    assert!(!open("## Open questions\n\n- None\n\n## Next\n\n- item"));
    assert!(!open("## Notes\n\n```\n## Open questions\n- in code\n```"));
    assert!(open(
        "## Открытые вопросы\n\n### Подраздел\n\nВопрос без списка"
    ));
}

#[test]
fn checks_the_whole_chain() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "docs/brd.md", &doc("approved", "", "text"));
    write(
        root,
        "docs/prd.md",
        &doc("approved", "[brd](brd.md)", "text"),
    );
    write(
        root,
        "docs/plan.md",
        &doc("approved", "[prd](prd.md), [brd](brd.md)", "text"),
    );
    let checked = document::check_chain(&[root.join("docs/plan.md")]).unwrap();
    assert_eq!(checked.len(), 3);

    write(
        root,
        "docs/brd.md",
        &doc("draft", "", "## Open questions\n\n- Budget?"),
    );
    write(
        root,
        "docs/prd.md",
        &doc("approved", "[brd](brd.md), [gone](gone.md)", "text"),
    );
    let error = document::check_chain(&[root.join("docs/plan.md")]).unwrap_err();
    assert!(matches!(error, Error::Invalid(_)));
    let text = error.to_string();
    for needle in [
        "brd.md has status draft",
        "brd.md has open questions",
        "gone.md does not exist",
    ] {
        assert!(text.contains(needle), "{text}");
    }
}

#[test]
fn survives_reference_cycles() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.md", &doc("approved", "[b](b.md)", "text"));
    write(dir.path(), "b.md", &doc("approved", "[a](a.md)", "text"));
    assert_eq!(
        document::check_chain(&[dir.path().join("a.md")])
            .unwrap()
            .len(),
        2
    );
    assert!(document::check_chain(&[]).unwrap().is_empty());
}
