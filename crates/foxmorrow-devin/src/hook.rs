use std::path::Path;

use foxmorrow_core::Result;
use serde::{Deserialize, Serialize};

const LAUNCH_CONTEXT: &str = "foxmorrow was requested with $fox. Invoke the skill `fox` (/foxmorrow:fox) before any other action; it runs the task through foxmorrow.";

#[derive(Debug, Default, Deserialize)]
pub struct HookInput {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub prompt_id: Option<String>,
}

pub fn contains_marker(prompt: &str) -> bool {
    ["$fox", "$foxmorrow"].iter().any(|marker| {
        prompt.match_indices(marker).any(|(start, _)| {
            let before = prompt[..start].chars().next_back();
            let after = prompt[start + marker.len()..].chars().next();
            before.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '$'))
                && after.is_none_or(|c| !(c.is_alphanumeric() || c == '-' || c == '_'))
        })
    })
}

pub fn read_session(project_root: &Path) -> Option<SessionRecord> {
    let text = std::fs::read_to_string(project_root.join(".foxmorrow").join("session")).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn handle(
    event: &str,
    input: &str,
    project_root: &Path,
    run_state: impl FnOnce(Option<&str>) -> Option<String>,
) -> Result<Option<String>> {
    let parsed: HookInput = serde_json::from_str(input)?;
    match event {
        "UserPromptSubmit" if parsed.prompt.as_deref().is_some_and(contains_marker) => {
            if let Some(session_id) = parsed.session_id {
                write_session(
                    project_root,
                    &SessionRecord {
                        session_id,
                        prompt_id: parsed.prompt_id,
                    },
                )?;
            }
            Ok(Some(context("UserPromptSubmit", LAUNCH_CONTEXT)))
        }
        "PostCompaction" if project_root.join(".foxmorrow").join("current").is_file() => {
            Ok(run_state(parsed.session_id.as_deref()).map(|state| {
                context(
                    "PostCompaction",
                    &format!("foxmorrow run state after compaction:\n{state}"),
                )
            }))
        }
        _ => Ok(None),
    }
}

fn write_session(project_root: &Path, record: &SessionRecord) -> Result<()> {
    let dir = project_root.join(".foxmorrow");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("session"), serde_json::to_string(record)?)?;
    Ok(())
}

fn context(event: &str, text: &str) -> String {
    serde_json::json!({ "hookSpecificOutput": { "hookEventName": event, "additionalContext": text } }).to_string()
}
