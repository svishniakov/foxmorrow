#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use foxmorrow_core::assignment::{self, Assignment, NewAssignment};
use foxmorrow_core::claims::{self, ChainObservation, ModelFamilies, ToolCall};
use foxmorrow_core::journal::Journal;
use foxmorrow_core::model::{Kind, VerificationState};
use foxmorrow_core::report;
use foxmorrow_core::run::{self, NewRun, Run};

pub const TASK: &str = "# Задача\n\n## Критерии приёмки\n\n- Первый критерий\n- Второй критерий\n";

pub fn new_run(dir: &Path) -> (Run, Journal) {
    new_run_with(dir, true)
}

pub fn new_run_with(dir: &Path, family_rule: bool) -> (Run, Journal) {
    run::create_run(NewRun {
        project_root: dir.to_path_buf(),
        criteria_path: "task.md".into(),
        criteria_heading: "Критерии приёмки".into(),
        criteria_markdown: TASK.into(),
        allowed_paths: vec!["src/".into(), "tests/".into()],
        baselines: Vec::new(),
        root_session_id: None,
        family_rule,
        skills_summary: Default::default(),
    })
    .unwrap()
}

pub fn spec(role: &str, kind: Kind, model: &str, depends_on: &[&str]) -> NewAssignment {
    NewAssignment {
        role: role.into(),
        kind,
        profile: Some(format!("foxmorrow:{role}")),
        expected_models: vec![model.into()],
        brief_sha256: Some(format!("brief-{role}")),
        role_sha256: Some(format!("role-{role}")),
        allowed_paths: vec!["src/".into()],
        depends_on: depends_on.iter().map(|id| id.to_string()).collect(),
        previous_id: None,
    }
}

pub fn root_spec() -> NewAssignment {
    NewAssignment {
        role: "root".into(),
        kind: Kind::Root,
        profile: None,
        expected_models: Vec::new(),
        brief_sha256: None,
        role_sha256: None,
        allowed_paths: vec!["src/".into()],
        depends_on: Vec::new(),
        previous_id: None,
    }
}

pub fn families() -> ModelFamilies {
    BTreeMap::from([
        ("swe-2-high".to_string(), "swe".to_string()),
        ("swe-2-low".to_string(), "swe".to_string()),
        ("gpt-6".to_string(), "gpt".to_string()),
        ("opus-5".to_string(), "claude".to_string()),
    ])
}

pub fn report_call(id: &str) -> ToolCall {
    ToolCall {
        id: format!("call-report-{id}"),
        name: "exec".into(),
        command: Some(format!("foxmorrow report -a {id} <<'JSON'")),
        exit_code: Some(0),
    }
}

pub fn observation(assignment: &Assignment, agent: &str, model: &str) -> ChainObservation {
    ChainObservation {
        source: "test".into(),
        agent_id: agent.into(),
        profile: assignment.profile.clone().unwrap_or_default(),
        task_text: format!(
            "foxmorrow assignment {} brief-sha256 {}",
            assignment.id,
            assignment.brief_sha256.clone().unwrap_or_default()
        ),
        system_texts: vec![format!(
            "foxmorrow-role-sha256: {}",
            assignment.role_sha256.clone().unwrap_or_default()
        )],
        models: vec![model.into()],
        tool_calls: vec![report_call(&assignment.id)],
    }
}

pub fn passed_report(criteria: &[&str]) -> String {
    let criteria: serde_json::Map<String, serde_json::Value> = criteria
        .iter()
        .map(|id| {
            (
                id.to_string(),
                serde_json::json!({"status": "passed", "reason": "checked"}),
            )
        })
        .collect();
    serde_json::json!({"summary": "done", "status": "completed", "outcome": "passed", "criteria": criteria})
        .to_string()
}

pub fn accept(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    agent: &str,
    model: &str,
    report_json: &str,
) -> Assignment {
    let created = assignment::create(journal, run, new).unwrap();
    report::submit(journal, run, &created.id, report_json).unwrap();
    let verification = claims::verify(
        journal,
        run,
        &created.id,
        Some(observation(&created, agent, model)),
        &families(),
    )
    .unwrap();
    assert_eq!(
        verification.state,
        VerificationState::Accepted,
        "{:?}",
        verification.claims
    );
    created
}

pub fn accepted_chain(journal: &mut Journal, run: &Run) -> (Assignment, Assignment, Assignment) {
    let author = accept(
        journal,
        run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
        "agent-author",
        "swe-2-high",
        &passed_report(&[]),
    );
    let review = accept(
        journal,
        run,
        spec("reviewer", Kind::Review, "gpt-6", &[&author.id]),
        "agent-review",
        "gpt-6",
        &passed_report(&[]),
    );
    let qa = accept(
        journal,
        run,
        spec("qa-verifier", Kind::Qa, "opus-5", &[&review.id]),
        "agent-qa",
        "opus-5",
        &passed_report(&["ac-1", "ac-2"]),
    );
    (author, review, qa)
}
