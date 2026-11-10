mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::catalog::{self, Catalog};
use foxmorrow_devin::agents::{self, MARKER, PROJECT_PREFIX, VERIFY_TOOLS};
use foxmorrow_devin::profile::{Profile, ProjectProfile};

#[test]
fn renders_devin_agent_files() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let catalog = Catalog::load(dir.path()).unwrap();
    let text = catalog::role_text(dir.path(), "reviewer").unwrap();
    let (content, sha) = agents::render(
        "reviewer",
        catalog.role("reviewer").unwrap(),
        "gpt-6-1-sol-high",
        &text,
    )
    .unwrap();
    let tools: String = VERIFY_TOOLS
        .iter()
        .map(|tool| format!("  - {tool}\n"))
        .collect();
    assert_eq!(
        content,
        format!(
            "---\nname: reviewer\ndescription: \"Reviewer\"\nmodel: gpt-6-1-sol-high\nallowed-tools:\n{tools}---\n\n{MARKER}{sha}\n\n{text}"
        )
    );
    assert_eq!(sha, catalog::role_sha256(&text));

    let backend = catalog::role_text(dir.path(), "backend-worker").unwrap();
    let (content, _) = agents::render(
        "backend-worker",
        catalog.role("backend-worker").unwrap(),
        "swe-2-high",
        &backend,
    )
    .unwrap();
    assert!(content.contains("description: \"Backend \\\"worker\\\"\"\n"));
    assert!(!content.contains("allowed-tools"));
}

#[test]
fn chooses_plugin_or_project_definitions() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let project = tempfile::tempdir().unwrap();
    let catalog = Catalog::load(dir.path()).unwrap();
    let profile = Profile::load(dir.path()).unwrap();

    let plugin_definition = agents::definition(
        dir.path(),
        project.path(),
        &catalog,
        &profile,
        &ProjectProfile::default(),
        "backend-worker",
    )
    .unwrap();
    assert_eq!(plugin_definition.profile, "foxmorrow:backend-worker");
    assert_eq!(
        plugin_definition.path,
        dir.path().join("agents/backend-worker.md")
    );
    assert_eq!(plugin_definition.model, "swe-2-high");

    write(
        project.path(),
        ".foxmorrow/profile.json",
        r#"{"roles": {"backend-worker": "claude-sonnet-5-5-high"}}"#,
    );
    let overrides = ProjectProfile::load(project.path()).unwrap();
    let project_definition = agents::definition(
        dir.path(),
        project.path(),
        &catalog,
        &profile,
        &overrides,
        "backend-worker",
    )
    .unwrap();
    assert_eq!(
        project_definition.profile,
        format!("{PROJECT_PREFIX}backend-worker")
    );
    assert_eq!(
        project_definition.path,
        project.path().join(".devin/agents/fox-backend-worker.md")
    );
    assert!(
        project_definition
            .content
            .contains("name: fox-backend-worker\n")
    );
    assert_eq!(
        project_definition.role_sha256,
        plugin_definition.role_sha256
    );

    write(
        project.path(),
        ".foxmorrow/profile.json",
        r#"{"roles": {"backend-worker": "kimi-k3"}}"#,
    );
    let unknown = ProjectProfile::load(project.path()).unwrap();
    assert!(matches!(
        agents::definition(
            dir.path(),
            project.path(),
            &catalog,
            &profile,
            &unknown,
            "backend-worker"
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn syncs_only_managed_files() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let catalog = Catalog::load(dir.path()).unwrap();
    let profile = Profile::load(dir.path()).unwrap();
    let definitions = agents::plugin_definitions(dir.path(), &catalog, &profile).unwrap();
    let agents_dir = dir.path().join("agents");

    let check = agents::sync(&definitions, &agents_dir, None, true).unwrap();
    assert_eq!(check.stale.len(), 2);
    assert!(check.written.is_empty());
    assert!(matches!(
        agents::ensure_current(&definitions[0]),
        Err(Error::Conflict(_))
    ));

    let written = agents::sync(&definitions, &agents_dir, None, false).unwrap();
    assert_eq!(written.written.len(), 2);
    assert!(agents::ensure_current(&definitions[0]).is_ok());
    assert!(
        agents::sync(&definitions, &agents_dir, None, true)
            .unwrap()
            .stale
            .is_empty()
    );

    write(
        dir.path(),
        "agents/old-role.md",
        &format!("---\nname: old-role\n---\n\n{MARKER}abc\n"),
    );
    write(
        dir.path(),
        "agents/handwritten.md",
        "---\nname: handwritten\n---\n",
    );
    let cleaned = agents::sync(&definitions, &agents_dir, None, false).unwrap();
    assert_eq!(cleaned.removed, vec![agents_dir.join("old-role.md")]);
    assert!(agents_dir.join("handwritten.md").is_file());

    let project = tempfile::tempdir().unwrap();
    write(
        project.path(),
        ".devin/agents/fox-gone.md",
        &format!("{MARKER}abc\n"),
    );
    write(
        project.path(),
        ".devin/agents/team-role.md",
        &format!("{MARKER}abc\n"),
    );
    let project_dir = project.path().join(".devin/agents");
    let none = agents::project_definitions(
        dir.path(),
        project.path(),
        &catalog,
        &profile,
        &ProjectProfile::default(),
    )
    .unwrap();
    assert!(none.is_empty());
    let report = agents::sync(&none, &project_dir, Some(PROJECT_PREFIX), false).unwrap();
    assert_eq!(report.removed, vec![project_dir.join("fox-gone.md")]);
    assert!(project_dir.join("team-role.md").is_file());
}
