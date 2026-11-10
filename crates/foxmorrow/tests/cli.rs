#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Setup {
    plugin: tempfile::TempDir,
    project: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        let setup = Setup {
            plugin: tempfile::tempdir().unwrap(),
            project: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        let plugin = setup.plugin.path();
        write(plugin, "roles/roles.json", CATALOG);
        write(plugin, "roles/_common.md", "## Common\n\nShared rules.\n");
        for role in ["backend-worker", "skilled-worker", "reviewer"] {
            write(plugin, &format!("roles/{role}.md"), &format!("# {role}\n"));
        }
        write(plugin, "profiles/devin.json", PROFILE);
        write(
            plugin,
            "templates/brief.en.md",
            "Role: {{role}} Profile: {{profile}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nfoxmorrow report -a {{assignment_id}}\n",
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(setup.project.path())
                .args(["init", "-q"])
                .status()
                .unwrap()
                .success()
        );
        write(
            setup.project.path(),
            "task.md",
            "# Task\n\n## Критерии приёмки\n\n- Health returns 200\n",
        );
        assert!(setup.run(&["sync", "--plugin"], None).status.success());
        setup
    }

    fn run(&self, args: &[&str], stdin: Option<&str>) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_foxmorrow"))
            .args(args)
            .current_dir(self.project.path())
            .env("FOXMORROW_PLUGIN_ROOT", self.plugin.path())
            .env("FOXMORROW_DEVIN_BIN", "/nonexistent/devin")
            .env("HOME", self.home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stdin.take().unwrap();
        if let Some(input) = stdin {
            pipe.write_all(input.as_bytes()).unwrap();
        }
        drop(pipe);
        child.wait_with_output().unwrap()
    }

    fn init(&self) -> serde_json::Value {
        json(&self.run(
            &[
                "init",
                "--criteria",
                "task.md#Критерии приёмки",
                "--paths",
                "src/",
            ],
            None,
        ))
    }

    fn agent(&self, relative: &str) -> PathBuf {
        self.plugin.path().join(relative)
    }
}

const CATALOG: &str = r#"{
  "version": 1,
  "source": {"repository": "test/agent-flow", "commit": "0000000"},
  "roles": {
    "backend-worker": {"description": "Backend", "tier": "worker", "kind": "author", "tools": "full"},
    "skilled-worker": {"description": "Skilled", "tier": "worker", "kind": "author", "tools": "full", "skills": {"required": ["special-skill"]}},
    "reviewer": {"description": "Reviewer", "tier": "verifier", "kind": "review", "tools": "verify"}
  }
}"#;

const PROFILE: &str = r#"{
  "version": 1, "brief_language": "en", "family_rule": true,
  "tiers": {"worker": "swe-2-high", "analyst": "claude-opus-5-5-high", "verifier": "gpt-6-1-sol-high", "escalation": "gpt-6-astra-xhigh"},
  "families": [{"family": "swe", "prefixes": ["swe-"]}, {"family": "claude", "prefixes": ["claude-"]}, {"family": "gpt", "prefixes": ["gpt-"]}]
}"#;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn text(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn failure(output: &Output, needle: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn init_records_profile_rule_and_missing_skills() {
    let setup = Setup::new();
    let init = setup.init();
    assert_eq!(init["criteria"][0]["id"], "ac-1");
    assert_eq!(init["family_rule"], true);
    assert_eq!(init["skills_source"], "approximate");
    assert_eq!(init["missing_skills"]["skilled-worker"][0], "special-skill");
}

#[test]
fn assign_prints_and_stores_the_brief() {
    let setup = Setup::new();
    setup.init();
    let brief = text(&setup.run(
        &[
            "assign",
            "--role",
            "backend-worker",
            "--goal",
            "Add GET /health",
            "--paths",
            "src/",
        ],
        None,
    ));
    assert!(
        brief.starts_with("foxmorrow assignment a-01 run r-"),
        "{brief}"
    );
    assert!(brief.contains("Role: backend-worker Profile: foxmorrow:backend-worker\n"));
    assert!(brief.contains("Goal: Add GET /health\n"));
    assert_eq!(text(&setup.run(&["brief", "-a", "a-01"], None)), brief);

    let status = json(&setup.run(&["status"], None));
    assert_eq!(status["assignments"][0]["role"], "backend-worker");

    let check = setup.run(
        &[
            "check",
            "-a",
            "a-01",
            "--",
            "sh",
            "-c",
            "echo hello; exit 3",
        ],
        None,
    );
    assert_eq!(check.status.code(), Some(3));
    let report =
        r#"{"summary":"tried","status":"completed","outcome":"failed","evidence":["chk-001"]}"#;
    json(&setup.run(&["report", "-a", "a-01"], Some(report)));
    failure(
        &setup.run(&["report", "-a", "a-01"], Some(report)),
        "already has a report",
    );
}

#[test]
fn assign_refuses_stale_roles_and_missing_required_skills() {
    let setup = Setup::new();
    setup.init();
    std::fs::write(setup.agent("agents/backend-worker.md"), "edited").unwrap();
    failure(
        &setup.run(
            &[
                "assign",
                "--role",
                "backend-worker",
                "--goal",
                "g",
                "--paths",
                "src/",
            ],
            None,
        ),
        "sync --plugin",
    );
    failure(
        &setup.run(
            &[
                "assign",
                "--role",
                "skilled-worker",
                "--goal",
                "g",
                "--paths",
                "src/",
            ],
            None,
        ),
        "missing required skills: special-skill",
    );
    let brief = text(&setup.run(
        &[
            "assign",
            "--role",
            "skilled-worker",
            "--goal",
            "g",
            "--paths",
            "src/",
            "--allow-missing-skills",
        ],
        None,
    ));
    assert!(brief.contains("Missing:\n- special-skill\n"));
    failure(
        &setup.run(
            &["assign", "--role", "backend-worker", "--paths", "src/"],
            None,
        ),
        "--goal",
    );
}

#[test]
fn project_overrides_need_sync_and_use_project_roles() {
    let setup = Setup::new();
    setup.init();
    write(
        setup.project.path(),
        ".foxmorrow/profile.json",
        r#"{"roles": {"backend-worker": "claude-sonnet-5-5-high"}}"#,
    );
    failure(
        &setup.run(
            &[
                "assign",
                "--role",
                "backend-worker",
                "--goal",
                "g",
                "--paths",
                "src/",
            ],
            None,
        ),
        "foxmorrow sync",
    );
    let check = setup.run(&["sync", "--check"], None);
    assert_eq!(check.status.code(), Some(1));
    json(&setup.run(&["sync"], None));
    assert!(
        setup
            .project
            .path()
            .join(".devin/agents/fox-backend-worker.md")
            .is_file()
    );
    let brief = text(&setup.run(
        &[
            "assign",
            "--role",
            "backend-worker",
            "--goal",
            "g",
            "--paths",
            "src/",
        ],
        None,
    ));
    assert!(brief.contains("Profile: fox-backend-worker\n"));
    assert!(setup.run(&["sync", "--check"], None).status.success());
}

#[test]
fn root_assignments_and_doctor() {
    let setup = Setup::new();
    setup.init();
    let root = json(&setup.run(&["assign", "--root", "--paths", "src/"], None));
    assert_eq!(root["kind"], "root");
    let doctor = json(&setup.run(&["doctor"], None));
    assert_eq!(doctor["roles"], 3);
    assert_eq!(doctor["stale_agents"].as_array().unwrap().len(), 0);
    std::fs::remove_file(setup.agent("agents/reviewer.md")).unwrap();
    assert_eq!(setup.run(&["doctor"], None).status.code(), Some(1));
}

#[test]
fn status_outside_a_run_fails() {
    let setup = Setup::new();
    failure(&setup.run(&["status"], None), "no foxmorrow run");
}
