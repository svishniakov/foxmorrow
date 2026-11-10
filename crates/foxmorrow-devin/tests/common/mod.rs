#![allow(dead_code)]

use std::path::Path;

pub const CATALOG: &str = r#"{
  "version": 1,
  "source": {"repository": "test/agent-flow", "commit": "0000000"},
  "roles": {
    "backend-worker": {"description": "Backend \"worker\"", "tier": "worker", "kind": "author", "tools": "full", "skills": {"optional": ["sql-queries"]}},
    "reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify", "skills": {"required": ["arch-simplicity-guard"]}}
  }
}"#;

pub const PROFILE: &str = r#"{
  "version": 1,
  "brief_language": "en",
  "family_rule": true,
  "tiers": {"worker": "swe-2-high", "analyst": "claude-opus-5-5-high", "verifier": "gpt-6-1-sol-high", "escalation": "gpt-6-astra-xhigh"},
  "roles": {"reviewer": "gpt-6-astra-high"},
  "families": [
    {"family": "swe", "prefixes": ["swe-"]},
    {"family": "claude", "prefixes": ["claude-"]},
    {"family": "gpt", "prefixes": ["gpt-"]}
  ]
}"#;

pub fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

pub fn plugin(root: &Path) {
    write(root, "roles/roles.json", CATALOG);
    write(root, "roles/_common.md", "## Common\n\nShared rules.\n");
    write(
        root,
        "roles/backend-worker.md",
        "# backend-worker\n\nBuild backends.\n",
    );
    write(root, "roles/reviewer.md", "# reviewer\n\nReview.\n");
    write(root, "profiles/devin.json", PROFILE);
}
