mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::catalog::Tier;
use foxmorrow_devin::profile::{ModelChoice, Profile, ProjectProfile};

#[test]
fn resolves_models_with_project_overrides_first() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let profile = Profile::load(dir.path()).unwrap();
    let none = ProjectProfile::default();
    assert_eq!(
        profile.model_for("backend-worker", Tier::Worker, &none),
        ModelChoice {
            model: "swe-2-high".into(),
            project_override: false
        }
    );
    assert_eq!(
        profile.model_for("reviewer", Tier::Verifier, &none).model,
        "gpt-6-astra-high"
    );

    write(
        dir.path(),
        ".foxmorrow/profile.json",
        r#"{"tiers": {"worker": "claude-sonnet-5-5-high"}, "roles": {"reviewer": "gpt-6-1-sol-low"}, "family_rule": false}"#,
    );
    let project = ProjectProfile::load(dir.path()).unwrap();
    assert_eq!(
        profile.model_for("backend-worker", Tier::Worker, &project),
        ModelChoice {
            model: "claude-sonnet-5-5-high".into(),
            project_override: true
        }
    );
    assert_eq!(
        profile
            .model_for("reviewer", Tier::Verifier, &project)
            .model,
        "gpt-6-1-sol-low"
    );
    assert!(profile.family_rule(&none));
    assert!(!profile.family_rule(&project));
}

#[test]
fn maps_models_to_families_by_prefix() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    let profile = Profile::load(dir.path()).unwrap();
    assert_eq!(profile.family_of("claude-haiku-5-5-low"), Some("claude"));
    assert_eq!(profile.family_of("kimi-k3"), None);
    let models = vec!["swe-2-medium".to_string(), "kimi-k3".to_string()];
    let families = profile.families_for(&models);
    assert_eq!(
        families.get("swe-2-medium").map(String::as_str),
        Some("swe")
    );
    assert!(!families.contains_key("kimi-k3"));
}

#[test]
fn rejects_incomplete_profiles() {
    let dir = tempfile::tempdir().unwrap();
    plugin(dir.path());
    write(
        dir.path(),
        "profiles/devin.json",
        &PROFILE.replace(
            r#""escalation": "gpt-6-astra-xhigh""#,
            r#""escalation_x": "gpt-6-astra-xhigh""#,
        ),
    );
    assert!(Profile::load(dir.path()).is_err());

    write(
        dir.path(),
        "profiles/devin.json",
        &PROFILE.replace(r#", "escalation": "gpt-6-astra-xhigh""#, ""),
    );
    assert!(matches!(Profile::load(dir.path()), Err(Error::Invalid(_))));

    write(
        dir.path(),
        "profiles/devin.json",
        &PROFILE.replace("gpt-6-astra-high", "kimi-k3"),
    );
    assert!(matches!(Profile::load(dir.path()), Err(Error::Invalid(_))));

    assert_eq!(
        ProjectProfile::load(tempfile::tempdir().unwrap().path()).unwrap(),
        ProjectProfile::default()
    );
}
