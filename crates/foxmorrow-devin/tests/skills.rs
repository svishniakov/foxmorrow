mod common;

use common::*;
use foxmorrow_devin::skills::{self, InventorySource};

#[test]
fn parses_devin_output_and_skips_broken_skills() {
    let names = skills::parse_devin_list(
        r#"[{"name": "astro", "errors": []}, {"name": "broken", "errors": ["bad frontmatter"]}, {"name": "bun"}]"#,
    )
    .unwrap();
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        vec!["astro".to_string(), "bun".to_string()]
    );
    assert!(skills::parse_devin_list("not json").is_none());
}

#[test]
fn falls_back_to_skill_directories() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write(project.path(), ".devin/skills/local-skill/SKILL.md", "x");
    write(home.path(), ".agents/skills/global-skill/SKILL.md", "x");
    write(home.path(), ".agents/skills/not-a-skill/README.md", "x");
    let found = skills::inventory(
        project.path(),
        std::path::Path::new("/nonexistent/devin"),
        home.path(),
    );
    assert_eq!(found.source, InventorySource::Approximate);
    assert!(found.names.contains("local-skill") && found.names.contains("global-skill"));
    assert!(!found.names.contains("not-a-skill"));
    assert_eq!(
        found.missing(&["global-skill".into(), "astro".into()]),
        vec!["astro".to_string()]
    );
}

#[cfg(unix)]
#[test]
fn asks_devin_when_available() {
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().unwrap();
    let script = bin.path().join("devin");
    std::fs::write(&script, "#!/bin/sh\n[ \"$1 $2 $3\" = \"skills list --json\" ] || exit 1\necho '[{\"name\": \"astro\", \"errors\": []}]'\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let project = tempfile::tempdir().unwrap();
    let found = skills::inventory(project.path(), &script, project.path());
    assert_eq!(found.source, InventorySource::Devin);
    assert_eq!(
        found.names.into_iter().collect::<Vec<_>>(),
        vec!["astro".to_string()]
    );
}
