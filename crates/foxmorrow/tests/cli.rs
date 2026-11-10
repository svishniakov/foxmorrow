#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use rusqlite::{Connection, OpenFlags, params};

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
        write(plugin, "schema-maps/devin.json", SCHEMA_MAP);
        write(
            plugin,
            "skills/fox/references/doc-flow.md",
            "# Doc Flow\n\n## PRD\n\n### PRD acceptance criteria\n\n- Scope is coherent\n- Status reflects facts\n\n## BRD\n\n### BRD acceptance criteria\n\n- Problem is stated\n",
        );
        write(
            plugin,
            "templates/brief.en.md",
            "Role: {{role}} Profile: {{profile}} role-sha256 {{role_sha256}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nfoxmorrow report -a {{assignment_id}}\n",
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

const SCHEMA_MAP: &str = include_str!("../../../schema-maps/devin.json");

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
    assert!(brief.contains("Role: backend-worker Profile: foxmorrow:backend-worker role-sha256 "));
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
    assert!(brief.contains("Profile: fox-backend-worker role-sha256 "));
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

struct SubChain<'a> {
    agent: &'a str,
    profile: &'a str,
    task: &'a str,
    role_hash: &'a str,
    model: &'a str,
    report_command: &'a str,
}

fn build_db(
    dir: &Path,
    session: &str,
    cwd: &str,
    root_model: &str,
    root_report: Option<&str>,
    subs: &[SubChain<'_>],
) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/devin-3000.12.43/sessions.db");
    let source = Connection::open_with_flags(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let ddl: Vec<String> = source
        .prepare(
            "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' AND type IN ('table', 'index')",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let path = dir.join("sessions.db");
    let db = Connection::open(&path).unwrap();
    for sql in ddl {
        db.execute_batch(&sql).unwrap();
    }
    db.execute(
        "INSERT INTO sessions (id, working_directory, backend_type, model, agent_mode, created_at, last_activity_at) VALUES (?1, ?2, 'windsurf', ?3, 'bypass', 0, 1)",
        params![session, cwd, root_model],
    )
    .unwrap();
    let message = |node: i64, parent: Option<i64>, json: serde_json::Value| {
        db.execute(
            "INSERT INTO message_nodes (session_id, node_id, parent_node_id, chat_message, created_at) VALUES (?1, ?2, ?3, ?4, 0)",
            params![session, node, parent, json.to_string()],
        )
        .unwrap();
    };
    let state = |id: &str, request: Option<serde_json::Value>, exit_code: Option<i64>| {
        db.execute(
            "INSERT INTO tool_call_state (session_id, tool_call_id, tool_call_json, tool_call_update_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                session,
                id,
                request.map(|value| value.to_string()),
                exit_code.map(|code| serde_json::json!({"_meta": {"terminal_exit": {"exit_code": code}}}).to_string())
            ],
        )
        .unwrap();
    };
    let mut calls: Vec<serde_json::Value> = subs
        .iter()
        .map(|sub| {
            serde_json::json!({"id": format!("rs-{}", sub.agent), "name": "run_subagent",
                "arguments": {"profile": sub.profile, "task": sub.task}})
        })
        .collect();
    if let Some(command) = root_report {
        calls.push(serde_json::json!({"id": "root-report", "name": "exec", "arguments": {"command": command}}));
        state("root-report", None, Some(0));
    }
    message(
        1,
        None,
        serde_json::json!({"role": "user", "content": "$fox implement"}),
    );
    message(
        2,
        Some(1),
        serde_json::json!({"role": "assistant", "content": "", "metadata": {"generation_model": root_model}, "tool_calls": calls}),
    );
    let mut last = 2;
    for (index, sub) in subs.iter().enumerate() {
        state(
            &format!("rs-{}", sub.agent),
            Some(serde_json::json!({"rawInput": {"profile": sub.profile, "task": sub.task}})),
            None,
        );
        last = 3 + index as i64;
        message(
            last,
            Some(last - 1),
            serde_json::json!({"role": "tool", "tool_call_id": format!("rs-{}", sub.agent),
                "content": format!("Subagent agent_id={} completed successfully:\n done", sub.agent)}),
        );
        let base = 100 * (index as i64 + 1);
        message(
            base,
            None,
            serde_json::json!({"role": "system", "content": format!("foxmorrow-role-sha256: {}\n\nrole text", sub.role_hash)}),
        );
        message(
            base + 1,
            Some(base),
            serde_json::json!({"role": "user", "content": sub.task}),
        );
        message(
            base + 2,
            Some(base + 1),
            serde_json::json!({"role": "assistant", "content": "", "metadata": {"generation_model": sub.model},
                "tool_calls": [{"id": format!("call-{}", sub.agent), "name": "exec", "arguments": {"command": sub.report_command}}]}),
        );
        state(&format!("call-{}", sub.agent), None, Some(0));
        message(
            base + 3,
            Some(base + 2),
            serde_json::json!({"role": "tool", "tool_call_id": format!("call-{}", sub.agent), "content": "ok"}),
        );
        message(
            base + 4,
            Some(base + 3),
            serde_json::json!({"role": "assistant", "content": "done", "metadata": {"generation_model": sub.model}}),
        );
        db.execute(
            "INSERT INTO subagent_heads (session_id, agent_id, chain_node_id, updated_at) VALUES (?1, ?2, ?3, 0)",
            params![session, sub.agent, base + 4],
        )
        .unwrap();
    }
    message(
        last + 1,
        Some(last),
        serde_json::json!({"role": "assistant", "content": "finished", "metadata": {"generation_model": root_model}}),
    );
    db.execute(
        "UPDATE sessions SET main_chain_id = ?1 WHERE id = ?2",
        params![last + 1, session],
    )
    .unwrap();
    path
}

fn db_arg(path: &Path) -> String {
    path.display().to_string()
}

fn brief_hash(brief: &str) -> String {
    brief
        .lines()
        .find(|line| line.starts_with("Role:"))
        .and_then(|line| line.split_whitespace().last())
        .unwrap()
        .to_string()
}

fn author_assignment(setup: &Setup) -> String {
    setup.run(
        &[
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--paths",
            "src/",
            "--session-id",
            "sess-1",
        ],
        None,
    );
    text(&setup.run(
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
    ))
}

const REPORT: &str = r#"{"summary":"done","status":"completed","outcome":"passed"}"#;

#[test]
fn verify_accepts_a_matching_chain_and_rejects_a_wrong_model() {
    let setup = Setup::new();
    let brief = author_assignment(&setup);
    json(&setup.run(&["report", "-a", "a-01"], Some(REPORT)));
    let hash = brief_hash(&brief);
    let sub = |model, dir: &Path| {
        build_db(
            dir,
            "sess-1",
            "/work",
            "claude-opus-5-5-high",
            None,
            &[SubChain {
                agent: "ag1",
                profile: "foxmorrow:backend-worker",
                task: &brief,
                role_hash: &hash,
                model,
                report_command: "bin/foxmorrow report -a a-01 <<'JSON'",
            }],
        )
    };

    let wrong_dir = tempfile::tempdir().unwrap();
    let wrong = sub("gpt-6-1-sol-high", wrong_dir.path());
    let rejected = setup.run(
        &["--sessions-db", &db_arg(&wrong), "verify", "-a", "a-01"],
        None,
    );
    assert_eq!(rejected.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&rejected.stdout).unwrap();
    assert_eq!(value["state"], "rejected");
    assert!(
        value["claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|claim| claim["id"] == "C3" && claim["state"] == "failed")
    );

    let good_dir = tempfile::tempdir().unwrap();
    let good = sub("swe-2-high", good_dir.path());
    let accepted = json(&setup.run(
        &["--sessions-db", &db_arg(&good), "verify", "-a", "a-01"],
        None,
    ));
    assert_eq!(accepted["state"], "accepted");
    assert_eq!(
        json(&setup.run(&["status"], None))["assignments"][0]["verification"],
        "accepted"
    );
}

#[test]
fn verify_without_native_evidence_is_unverified() {
    let setup = Setup::new();
    author_assignment(&setup);
    json(&setup.run(&["report", "-a", "a-01"], Some(REPORT)));
    let output = setup.run(
        &[
            "--sessions-db",
            "/nonexistent/sessions.db",
            "verify",
            "-a",
            "a-01",
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["state"], "unverified");
    assert!(String::from_utf8_lossy(&output.stderr).contains("no native evidence"));
}

#[test]
fn root_stop_is_verified_and_finalized_with_the_root_model() {
    let setup = Setup::new();
    let dir = tempfile::tempdir().unwrap();
    let db = build_db(
        dir.path(),
        "sess-1",
        "/work",
        "claude-opus-5-5-high",
        Some("bin/foxmorrow report -a a-01 <<'JSON'"),
        &[],
    );
    let db = db_arg(&db);
    let init = json(&setup.run(
        &[
            "--sessions-db",
            &db,
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--paths",
            "src/",
            "--session-id",
            "sess-1",
        ],
        None,
    ));
    assert_eq!(init["root_session_id"], "sess-1");
    assert_eq!(init["root_model"], "claude-opus-5-5-high");
    assert_eq!(
        json(&setup.run(&["status"], None))["root_model"],
        "claude-opus-5-5-high"
    );

    json(&setup.run(&["assign", "--root", "--paths", "src/"], None));
    let stop = r#"{"summary":"stopped","status":"completed","outcome":"unverified","reason":"no browser access","unverified":["ac-1"]}"#;
    json(&setup.run(&["report", "-a", "a-01"], Some(stop)));
    assert_eq!(
        json(&setup.run(&["--sessions-db", &db, "verify", "-a", "a-01"], None))["state"],
        "accepted"
    );
    let record = json(&setup.run(
        &["--sessions-db", &db, "finalize", "--report", "a-01"],
        None,
    ));
    assert_eq!(record["outcome"], "unverified");
    assert_eq!(record["root_model"], "claude-opus-5-5-high");
    assert_eq!(
        json(&setup.run(&["status"], None))["final_outcome"],
        "unverified"
    );
}

#[test]
fn hooks_record_the_session_and_report_state() {
    let setup = Setup::new();
    let marked = text(&setup.run(
        &["hook", "UserPromptSubmit"],
        Some(r#"{"session_id":"sess-1","prompt_id":"p-1","prompt":"$fox implement the plan"}"#),
    ));
    assert!(marked.contains("skill `fox`"), "{marked}");
    let session = std::fs::read_to_string(setup.project.path().join(".foxmorrow/session")).unwrap();
    assert!(session.contains("sess-1"));

    let quiet = setup.run(
        &["hook", "UserPromptSubmit"],
        Some(r#"{"session_id":"sess-1","prompt":"hello"}"#),
    );
    assert!(quiet.status.success() && quiet.stdout.is_empty());

    json(&setup.run(
        &[
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--paths",
            "src/",
        ],
        None,
    ));
    let state = text(&setup.run(
        &["hook", "PostCompaction"],
        Some(r#"{"session_id":"sess-1"}"#),
    ));
    assert!(state.contains("run_id"), "{state}");
    let foreign = setup.run(
        &["hook", "PostCompaction"],
        Some(r#"{"session_id":"other"}"#),
    );
    assert!(foreign.status.success() && foreign.stdout.is_empty());

    let broken = setup.run(&["hook", "UserPromptSubmit"], Some("not json"));
    assert!(broken.status.success() && broken.stdout.is_empty());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("hook UserPromptSubmit"));
}

fn document_text(status: &str, based_on: &str, body: &str) -> String {
    format!("# Doc\n\nDocument status: {status}\nBased on: {based_on}\n\n{body}\n")
}

#[test]
fn init_plan_requires_an_approved_chain() {
    let setup = Setup::new();
    let root = setup.project.path();
    write(
        root,
        "docs/prd.md",
        &document_text("approved", "", "## Scope\n\n- health"),
    );
    write(
        root,
        "docs/plan.md",
        &document_text(
            "approved",
            "[prd](prd.md)",
            "## Критерии приёмки\n\n- Health returns 200\n- Tests pass",
        ),
    );
    let init = json(&setup.run(&["init", "--plan", "docs/plan.md", "--paths", "src/"], None));
    assert_eq!(init["criteria"][1]["text"], "Tests pass");
    assert_eq!(
        json(&setup.run(&["status"], None))["criteria_path"],
        "docs/plan.md"
    );

    write(
        root,
        "docs/prd.md",
        &document_text("draft", "", "## Open questions\n\n- Which port?"),
    );
    let refused = setup.run(&["init", "--plan", "docs/plan.md", "--paths", "src/"], None);
    failure(&refused, "prd.md has status draft");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("prd.md has open questions"));

    write(
        root,
        "docs/en.md",
        &document_text("approved", "", "## Acceptance criteria\n\n- Works"),
    );
    assert_eq!(
        json(&setup.run(&["init", "--plan", "docs/en.md", "--paths", "src/"], None))["criteria"][0]
            ["text"],
        "Works"
    );
    write(
        root,
        "docs/none.md",
        &document_text("approved", "", "## Notes\n\n- nothing"),
    );
    failure(
        &setup.run(&["init", "--plan", "docs/none.md", "--paths", "src/"], None),
        "Критерии приёмки",
    );
}

#[test]
fn init_doc_uses_type_criteria_and_checks_the_basis() {
    let setup = Setup::new();
    let root = setup.project.path();
    let brd = json(&setup.run(&["init", "--doc", "brd", "--paths", "docs/brd.md"], None));
    assert_eq!(brd["criteria"][0]["text"], "Problem is stated");

    failure(
        &setup.run(&["init", "--doc", "prd", "--paths", "docs/prd.md"], None),
        "--based-on",
    );
    write(root, "docs/brd.md", &document_text("draft", "", "text"));
    failure(
        &setup.run(
            &[
                "init",
                "--doc",
                "prd",
                "--based-on",
                "docs/brd.md",
                "--paths",
                "docs/prd.md",
            ],
            None,
        ),
        "has status draft",
    );
    write(root, "docs/brd.md", &document_text("approved", "", "text"));
    let prd = json(&setup.run(
        &[
            "init",
            "--doc",
            "prd",
            "--based-on",
            "docs/brd.md",
            "--paths",
            "docs/prd.md",
        ],
        None,
    ));
    assert_eq!(prd["criteria"][0]["text"], "Scope is coherent");
    let status = json(&setup.run(&["status"], None));
    assert!(
        status["criteria_path"]
            .as_str()
            .unwrap()
            .ends_with("skills/fox/references/doc-flow.md")
    );
}

#[test]
fn init_needs_exactly_one_criteria_source() {
    let setup = Setup::new();
    assert_eq!(
        setup.run(&["init", "--paths", "src/"], None).status.code(),
        Some(2)
    );
    let both = setup.run(
        &[
            "init",
            "--criteria",
            "task.md#Критерии приёмки",
            "--doc",
            "brd",
            "--paths",
            "src/",
        ],
        None,
    );
    assert_eq!(both.status.code(), Some(2));
    assert_eq!(
        setup
            .run(
                &[
                    "init",
                    "--based-on",
                    "x.md",
                    "--criteria",
                    "task.md#Критерии приёмки",
                    "--paths",
                    "src/"
                ],
                None
            )
            .status
            .code(),
        Some(2)
    );
}
