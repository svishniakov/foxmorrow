use std::collections::BTreeSet;
use std::path::PathBuf;

use foxmorrow_core::brief;
use foxmorrow_core::catalog::{self, Catalog};
use foxmorrow_core::criteria;
use foxmorrow_core::model::Kind;
use foxmorrow_devin::agents;
use foxmorrow_devin::profile::{Profile, ProjectProfile};

const ROLES: [&str; 21] = [
    "product-manager",
    "architect",
    "supervising-architect",
    "researcher",
    "documenter",
    "ai-slops-hunter",
    "marketing-growth-strategist",
    "rag-retrieval-engineer",
    "astro-worker",
    "backend-worker",
    "bun-worker",
    "devops-worker",
    "elixir-phoenix-worker",
    "frontend-worker",
    "golang-worker",
    "ios-worker",
    "python-worker",
    "typescript-worker",
    "reviewer",
    "qa-verifier",
    "senior-qa-verifier",
];

const FORBIDDEN: [&str; 16] = [
    "codex",
    "traceable-runs",
    "agent_config",
    "native response",
    "native evidence",
    "native action",
    "delegation packet",
    "orchestrator",
    "journal-backed",
    "verification-only",
    "chat delivery",
    "restricted to chat",
    "spawn_agent",
    "../references/",
    "flows.md",
    "implementation-plan-authoring",
];

const PLACEHOLDERS: [&str; 14] = [
    "foxmorrow",
    "assignment_id",
    "role",
    "profile",
    "role_sha256",
    "expected_models",
    "references_dir",
    "skills",
    "missing_skills",
    "goal",
    "criteria_source",
    "criteria",
    "allowed_paths",
    "context",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn catalog_has_exactly_the_v1_roles() {
    let catalog = Catalog::load(&root()).unwrap();
    let names: BTreeSet<&str> = catalog.roles.keys().map(String::as_str).collect();
    assert_eq!(names, ROLES.into_iter().collect());
}

#[test]
fn verifiers_never_share_a_model_family_with_authors() {
    let root = root();
    let catalog = Catalog::load(&root).unwrap();
    let profile = Profile::load(&root).unwrap();
    let none = ProjectProfile::default();
    let family = |role: &str| {
        let spec = catalog.role(role).unwrap();
        profile
            .family_of(&profile.model_for(role, spec.tier, &none).model)
            .unwrap()
            .to_string()
    };
    for (verifier, spec) in catalog
        .roles
        .iter()
        .filter(|(_, spec)| spec.kind.is_verification())
    {
        for (author, _) in catalog
            .roles
            .iter()
            .filter(|(_, spec)| spec.kind == Kind::Author)
        {
            assert_ne!(
                family(verifier),
                family(author),
                "{verifier} and {author} share a model family"
            );
        }
        assert_eq!(spec.tools, catalog::ToolSet::Verify);
    }
}

#[test]
fn generated_plugin_agents_are_current() {
    let root = root();
    let catalog = Catalog::load(&root).unwrap();
    let profile = Profile::load(&root).unwrap();
    let definitions = agents::plugin_definitions(&root, &catalog, &profile).unwrap();
    let report = agents::sync(&definitions, &root.join("agents"), None, true).unwrap();
    assert!(
        report.stale.is_empty(),
        "run foxmorrow sync --plugin: {:?}",
        report.stale
    );
}

#[test]
fn role_texts_are_ported_cleanly() {
    let root = root();
    for role in ROLES {
        let text = catalog::role_text(&root, role).unwrap();
        assert!(text.starts_with("# "), "{role} must start with its heading");
        let lower = text.to_lowercase();
        for term in FORBIDDEN {
            assert!(!lower.contains(term), "{role} still contains '{term}'");
        }
        for (index, _) in text.match_indices("references/") {
            let rest = &text[index + "references/".len()..];
            assert!(
                rest.starts_with("doc-flow.md") || rest.starts_with("contract.md"),
                "{role} links to an unknown reference: {}",
                &rest[..rest.len().min(40)]
            );
        }
    }
}

#[test]
fn brief_templates_agree_on_commands_and_placeholders() {
    let root = root();
    let commands = |language: &str| -> Vec<String> {
        let template = brief::load_template(&root, language).unwrap();
        for placeholder in PLACEHOLDERS {
            assert!(
                template.contains(&format!("{{{{{placeholder}}}}}")),
                "{language} lacks {placeholder}"
            );
        }
        template
            .lines()
            .map(str::trim)
            .filter(|line| {
                line.starts_with("{{foxmorrow}} ")
                    || (line.starts_with('{') && !line.starts_with("{{"))
                    || line.starts_with("\"criteria\"")
                    || line.starts_with("\"evidence\"")
            })
            .map(str::to_string)
            .collect()
    };
    let english = commands("en");
    assert_eq!(english.len(), 5);
    assert_eq!(english, commands("ru"));
}

fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

fn anchors(markdown: &str) -> BTreeSet<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let depth = line.bytes().take_while(|byte| *byte == b'#').count();
            ((1..=6).contains(&depth))
                .then(|| line[depth..].trim())
                .filter(|title| !title.is_empty())
        })
        .map(slug)
        .collect()
}

#[test]
fn skill_has_frontmatter_and_existing_references() {
    let root = root();
    let skill = std::fs::read_to_string(root.join("skills/fox/SKILL.md")).unwrap();
    let frontmatter = skill
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .unwrap()
        .0;
    assert!(frontmatter.lines().any(|line| line == "name: fox"));
    assert!(frontmatter.lines().any(|line| {
        line.strip_prefix("description: ")
            .is_some_and(|text| !text.trim().is_empty())
    }));
    for reference in ["orchestrator.md", "doc-flow.md", "contract.md"] {
        assert!(
            skill.contains(&format!("references/{reference}")),
            "SKILL.md does not link {reference}"
        );
        assert!(root.join("skills/fox/references").join(reference).is_file());
    }
}

#[test]
fn role_links_point_to_existing_doc_flow_sections() {
    let root = root();
    let doc_flow =
        anchors(&std::fs::read_to_string(root.join("skills/fox/references/doc-flow.md")).unwrap());
    for role in ROLES {
        let text = catalog::role_text(&root, role).unwrap();
        for (index, _) in text.match_indices("references/doc-flow.md#") {
            let anchor: String = text[index + "references/doc-flow.md#".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            assert!(
                doc_flow.contains(&anchor),
                "{role} links to a missing doc-flow section #{anchor}"
            );
        }
    }
}

#[test]
fn every_document_type_has_parsable_criteria() {
    let markdown =
        std::fs::read_to_string(root().join("skills/fox/references/doc-flow.md")).unwrap();
    for kind in ["BRD", "ADR", "PRD", "Plan"] {
        let (_, found) =
            criteria::parse_section(&markdown, &format!("{kind} acceptance criteria")).unwrap();
        assert!(found.len() >= 4, "{kind} has {} criteria", found.len());
    }
}

const TARGETS: [&str; 5] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-pc-windows-msvc",
];

fn json_file(relative: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(relative)).unwrap()).unwrap()
}

#[test]
fn versions_agree_across_the_plugin() {
    let cargo = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let version = cargo
        .lines()
        .skip_while(|line| *line != "[workspace.package]")
        .find_map(|line| line.strip_prefix("version = \""))
        .map(|rest| rest.trim_end_matches('"').to_string())
        .unwrap();
    let manifest = json_file(".devin-plugin/plugin.json");
    let checksums = json_file("checksums.json");
    assert_eq!(manifest["name"], "foxmorrow");
    assert_eq!(manifest["version"], version.as_str());
    assert_eq!(checksums["version"], version.as_str());
    assert_eq!(checksums["repository"], "svishniakov/foxmorrow");
    for (target, sha) in checksums["binaries"].as_object().unwrap() {
        assert!(
            TARGETS.contains(&target.as_str()),
            "unknown target {target}"
        );
        let sha = sha.as_str().unwrap();
        assert!(
            sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()),
            "{target}: {sha}"
        );
    }
}

#[test]
fn hooks_call_the_loader_for_handled_events() {
    let hooks = json_file("hooks.json");
    let events = hooks.as_object().unwrap();
    let names: Vec<&str> = events.keys().map(String::as_str).collect();
    assert_eq!(names, vec!["PostCompaction", "UserPromptSubmit"]);
    for (event, entries) in events {
        for entry in entries.as_array().unwrap() {
            for hook in entry["hooks"].as_array().unwrap() {
                assert_eq!(hook["type"], "command");
                assert_eq!(
                    hook["command"],
                    format!("${{PLUGIN_ROOT}}/bin/foxmorrow hook {event}")
                );
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn launchers_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    for relative in ["bin/foxmorrow", "scripts/write-checksums.sh"] {
        let mode = std::fs::metadata(root().join(relative))
            .unwrap()
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "{relative} is not executable");
    }
}
