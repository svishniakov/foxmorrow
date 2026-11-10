use std::path::Path;

use foxmorrow_devin::hook::{self, SessionRecord, contains_marker, read_session};

fn run_hook(
    event: &str,
    input: &str,
    root: &Path,
    state: Option<&str>,
) -> foxmorrow_core::Result<Option<String>> {
    hook::handle(event, input, root, |_| state.map(str::to_string))
}

fn context(output: &str) -> serde_json::Value {
    serde_json::from_str(output).unwrap()
}

#[test]
fn recognizes_the_launch_markers() {
    for prompt in [
        "$fox do it",
        "Please $fox: build",
        "($foxmorrow) build",
        "do it\n$fox",
        "$fox",
        "$foxmorrow",
    ] {
        assert!(contains_marker(prompt), "{prompt}");
    }
    for prompt in [
        "$foxes",
        "$foxmorrow2",
        "a$fox",
        "$$fox",
        "fox",
        "$fox-trot",
        "use $foxy",
    ] {
        assert!(!contains_marker(prompt), "{prompt}");
    }
}

#[test]
fn a_marked_prompt_records_the_session_and_asks_for_the_skill() {
    let dir = tempfile::tempdir().unwrap();
    let output = run_hook(
        "UserPromptSubmit",
        r#"{"session_id":"s-1","prompt_id":"p-1","prompt":"$fox implement"}"#,
        dir.path(),
        None,
    )
    .unwrap()
    .unwrap();
    let value = context(&output);
    assert_eq!(
        value["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    assert!(
        value["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("skill `fox`")
    );
    assert_eq!(
        read_session(dir.path()),
        Some(SessionRecord {
            session_id: "s-1".into(),
            prompt_id: Some("p-1".into())
        })
    );
}

#[test]
fn unmarked_prompts_leave_no_trace() {
    let dir = tempfile::tempdir().unwrap();
    let output = run_hook(
        "UserPromptSubmit",
        r#"{"session_id":"s-1","prompt":"hello"}"#,
        dir.path(),
        None,
    )
    .unwrap();
    assert_eq!(output, None);
    assert!(!dir.path().join(".foxmorrow").exists());
}

#[test]
fn compaction_returns_state_only_for_the_known_run() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        run_hook(
            "PostCompaction",
            r#"{"session_id":"s-1"}"#,
            dir.path(),
            Some("{}")
        )
        .unwrap(),
        None
    );
    std::fs::create_dir_all(dir.path().join(".foxmorrow")).unwrap();
    std::fs::write(dir.path().join(".foxmorrow/current"), "r-1\n").unwrap();
    let output = run_hook(
        "PostCompaction",
        r#"{"session_id":"s-1"}"#,
        dir.path(),
        Some("{\"run_id\":\"r-1\"}"),
    )
    .unwrap()
    .unwrap();
    let value = context(&output);
    assert_eq!(
        value["hookSpecificOutput"]["hookEventName"],
        "PostCompaction"
    );
    assert!(
        value["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap()
            .contains("r-1")
    );
    assert_eq!(
        run_hook(
            "PostCompaction",
            r#"{"session_id":"s-1"}"#,
            dir.path(),
            None
        )
        .unwrap(),
        None
    );
}

#[test]
fn other_events_and_bad_input() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        run_hook("SessionEnd", "{}", dir.path(), None).unwrap(),
        None
    );
    assert!(run_hook("UserPromptSubmit", "not json", dir.path(), None).is_err());
}
