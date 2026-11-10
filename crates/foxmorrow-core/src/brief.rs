use std::path::Path;

use crate::assignment::Assignment;
use crate::error::{Result, invalid};
use crate::model::Outcome;
use crate::run::Run;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextItem {
    pub assignment_id: String,
    pub role: String,
    pub outcome: Outcome,
    pub summary: String,
}

pub struct BriefInput<'a> {
    pub run: &'a Run,
    pub assignment: &'a Assignment,
    pub references_dir: &'a str,
    pub foxmorrow: &'a str,
    pub skills: &'a [String],
    pub missing_skills: &'a [String],
    pub goal: &'a str,
    pub context: &'a [ContextItem],
}

pub fn render(template: &str, input: &BriefInput<'_>) -> Result<String> {
    if !template.contains("{{assignment_id}}") {
        return invalid("brief template must contain {{assignment_id}}");
    }
    let goal = input.goal.trim();
    if goal.is_empty() {
        return invalid("brief goal must not be empty");
    }
    let assignment = input.assignment;
    let values: Vec<(&str, String)> = vec![
        ("assignment_id", assignment.id.clone()),
        ("role", assignment.role.clone()),
        ("profile", assignment.profile.clone().unwrap_or_default()),
        (
            "role_sha256",
            assignment.role_sha256.clone().unwrap_or_default(),
        ),
        ("expected_models", assignment.expected_models.join(", ")),
        ("references_dir", input.references_dir.to_string()),
        ("foxmorrow", input.foxmorrow.to_string()),
        ("skills", list(input.skills.iter().cloned())),
        ("missing_skills", list(input.missing_skills.iter().cloned())),
        ("goal", goal.to_string()),
        (
            "criteria_source",
            format!("{}#{}", input.run.criteria_path, input.run.criteria_heading),
        ),
        (
            "criteria",
            list(
                input
                    .run
                    .criteria
                    .iter()
                    .map(|criterion| format!("{}: {}", criterion.id, criterion.text)),
            ),
        ),
        (
            "allowed_paths",
            list(assignment.allowed_paths.iter().cloned()),
        ),
        (
            "context",
            list(input.context.iter().map(|item| {
                format!(
                    "{} {} {}: {}",
                    item.assignment_id,
                    item.role,
                    item.outcome.as_str(),
                    item.summary
                )
            })),
        ),
    ];
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return invalid("unclosed brief placeholder");
        };
        let name = &after[..end];
        let Some((_, value)) = values.iter().find(|(known, _)| *known == name) else {
            return invalid(format!("unknown brief placeholder {{{{{name}}}}}"));
        };
        output.push_str(value);
        rest = &after[end + 2..];
    }
    output.push_str(rest);
    Ok(output)
}

pub fn load_template(plugin_root: &Path, language: &str) -> Result<String> {
    if language.is_empty()
        || !language
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return invalid(format!(
            "brief language '{language}' must use letters, digits and hyphens"
        ));
    }
    let templates = plugin_root.join("templates");
    if let Ok(text) = std::fs::read_to_string(templates.join(format!("brief.{language}.md"))) {
        return Ok(text);
    }
    let mut available: Vec<String> = std::fs::read_dir(&templates)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| {
            name.strip_prefix("brief.")
                .and_then(|rest| rest.strip_suffix(".md"))
                .map(str::to_string)
        })
        .collect();
    available.sort();
    invalid(format!(
        "no brief template for language '{language}'; available: {}",
        available.join(", ")
    ))
}

fn list(items: impl Iterator<Item = String>) -> String {
    let lines: Vec<String> = items.map(|item| format!("- {item}")).collect();
    if lines.is_empty() {
        "-".to_string()
    } else {
        lines.join("\n")
    }
}
