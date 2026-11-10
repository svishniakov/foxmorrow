use std::path::Path;

use foxmorrow_core::Error;
use foxmorrow_core::catalog::{self, Catalog, Tier, ToolSet};
use foxmorrow_core::model::Kind;
use foxmorrow_core::util::sha256_hex;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn plugin(root: &Path, roles: &str) {
    write(
        root,
        "roles/roles.json",
        &format!(
            r#"{{"version": 1, "source": {{"repository": "test/agent-flow", "commit": "0000000"}}, "roles": {{{roles}}}}}"#
        ),
    );
    write(root, "roles/_common.md", "## Common\n\nShared rules.\n");
    write(
        root,
        "roles/backend-worker.md",
        "# backend-worker\n\nBuild backends.\n\n",
    );
    write(root, "roles/reviewer.md", "# reviewer\n\nReview.\n");
}

const BACKEND: &str = r#""backend-worker": {"description": "Backend", "tier": "worker", "kind": "author", "tools": "full", "skills": {"optional": ["sql-queries"]}}"#;
const REVIEWER: &str = r#""reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify", "skills": {"required": ["arch-simplicity-guard"]}}"#;

#[test]
fn loads_roles_and_joins_role_text_with_common_rules() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &format!("{BACKEND}, {REVIEWER}"));
    let catalog = Catalog::load(dir.path()).unwrap();
    let reviewer = catalog.role("reviewer").unwrap();
    assert_eq!(
        (reviewer.tier, reviewer.kind, reviewer.tools),
        (Tier::Verifier, Kind::Review, ToolSet::Verify)
    );
    assert_eq!(
        reviewer.skills.required,
        vec!["arch-simplicity-guard".to_string()]
    );
    assert!(
        catalog
            .role("backend-worker")
            .unwrap()
            .skills
            .required
            .is_empty()
    );
    let text = catalog::role_text(dir.path(), "backend-worker").unwrap();
    assert_eq!(
        text,
        "# backend-worker\n\nBuild backends.\n\n## Common\n\nShared rules.\n"
    );
    assert_eq!(catalog::role_sha256(&text), sha256_hex(text.as_bytes()));
    assert!(matches!(catalog.role("designer"), Err(Error::NotFound(_))));
    assert_eq!(Tier::Escalation.as_str(), "escalation");
}

#[test]
fn verification_roles_cannot_edit() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &REVIEWER.replace("\"verify\"", "\"full\""));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));
}

#[test]
fn rejects_missing_texts_bad_names_and_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    plugin(
        dir.path(),
        &BACKEND.replace("backend-worker", "python-worker"),
    );
    assert!(matches!(Catalog::load(dir.path()), Err(Error::NotFound(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("backend-worker", "../escape"));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(
        dir.path(),
        &BACKEND.replace("\"tools\"", "\"model\": \"x\", \"tools\""),
    );
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Json(_))));

    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path(), &BACKEND.replace("\"author\"", "\"root\""));
    assert!(matches!(Catalog::load(dir.path()), Err(Error::Invalid(_))));
}
