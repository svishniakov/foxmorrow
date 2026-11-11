mod common;

use common::*;
use foxmorrow_core::Error;
use foxmorrow_core::assignment;
use foxmorrow_core::brief::{self, BriefInput, ContextItem};
use foxmorrow_core::model::{Kind, Outcome};

const TEMPLATE: &str = "Role: {{role}} Profile: {{profile}} role-sha256 {{role_sha256}}\nModel: {{expected_models}}\nReferences: {{references_dir}}\nSkills:\n{{skills}}\nMissing:\n{{missing_skills}}\nGoal: {{goal}}\nCriteria ({{criteria_source}}):\n{{criteria}}\nPaths:\n{{allowed_paths}}\nContext:\n{{context}}\n{{foxmorrow}} report -a {{assignment_id}}\n";

fn input_parts(
    dir: &std::path::Path,
) -> (
    foxmorrow_core::run::Run,
    foxmorrow_core::assignment::Assignment,
) {
    let (run, mut journal) = new_run(dir);
    let created = assignment::create(
        &mut journal,
        &run,
        spec("backend-worker", Kind::Author, "swe-2-high", &[]),
    )
    .unwrap();
    (run, created)
}

#[test]
fn fills_every_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let (run, created) = input_parts(dir.path());
    let context = vec![ContextItem {
        assignment_id: "a-00".into(),
        role: "architect".into(),
        outcome: Outcome::Passed,
        summary: "Plan agreed".into(),
    }];
    let skills = vec!["sql-queries".to_string()];
    let text = brief::render(
        TEMPLATE,
        &BriefInput {
            run: &run,
            assignment: &created,
            references_dir: "/plugin/skills/fox/references",
            foxmorrow: "/plugin/bin/foxmorrow",
            skills: &skills,
            missing_skills: &[],
            memory: &[],
            goal: "  Add GET /health {{not a placeholder}}  ",
            context: &context,
        },
    )
    .unwrap();
    assert_eq!(
        text,
        "Role: backend-worker Profile: foxmorrow:backend-worker role-sha256 role-backend-worker\n\
Model: swe-2-high\n\
References: /plugin/skills/fox/references\n\
Skills:\n- sql-queries\n\
Missing:\n-\n\
Goal: Add GET /health {{not a placeholder}}\n\
Criteria (task.md#Критерии приёмки):\n- ac-1: Первый критерий\n- ac-2: Второй критерий\n\
Paths:\n- src/\n\
Context:\n- a-00 architect passed: Plan agreed\n\
/plugin/bin/foxmorrow report -a a-01\n"
    );
}

#[test]
fn rejects_broken_templates_and_empty_goals() {
    let dir = tempfile::tempdir().unwrap();
    let (run, created) = input_parts(dir.path());
    let render = |template: &str, goal: &str| {
        brief::render(
            template,
            &BriefInput {
                run: &run,
                assignment: &created,
                references_dir: "/r",
                foxmorrow: "/plugin/bin/foxmorrow",
                skills: &[],
                missing_skills: &[],
                memory: &[],
                goal,
                context: &[],
            },
        )
    };
    assert!(matches!(
        render("{{assignment_id}} {{unknown}}", "g"),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        render("{{assignment_id}} {{role", "g"),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(render("no id here", "g"), Err(Error::Invalid(_))));
    assert!(matches!(
        render("{{assignment_id}}", "  "),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn loads_templates_by_language() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("templates")).unwrap();
    std::fs::write(
        dir.path().join("templates/brief.en.md"),
        "EN {{assignment_id}}",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("templates/brief.ru.md"),
        "RU {{assignment_id}}",
    )
    .unwrap();
    assert_eq!(
        brief::load_template(dir.path(), "ru").unwrap(),
        "RU {{assignment_id}}"
    );
    let error = brief::load_template(dir.path(), "de").unwrap_err();
    assert!(error.to_string().contains("available: en, ru"), "{error}");
    assert!(matches!(
        brief::load_template(dir.path(), "../en"),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn renders_project_memory_entries() {
    let dir = tempfile::tempdir().unwrap();
    let (run, created) = input_parts(dir.path());
    let memory = vec!["[r-1 a-02] Cite only receipts of your own assignment.".to_string()];
    let text = brief::render(
        "{{assignment_id}}\nMemory:\n{{memory}}\n",
        &BriefInput {
            run: &run,
            assignment: &created,
            references_dir: "/r",
            foxmorrow: "/plugin/bin/foxmorrow",
            skills: &[],
            missing_skills: &[],
            memory: &memory,
            goal: "g",
            context: &[],
        },
    )
    .unwrap();
    assert_eq!(
        text,
        "a-01\nMemory:\n- [r-1 a-02] Cite only receipts of your own assignment.\n"
    );
}
