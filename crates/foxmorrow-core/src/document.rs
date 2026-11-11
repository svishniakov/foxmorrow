use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result, invalid};

const QUESTION_HEADINGS: [&str; 2] = ["открытые вопросы", "open questions"];
const NONE_MARKERS: [&str; 5] = [
    "нет",
    "none",
    "нет открытых вопросов",
    "no open questions",
    "—",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: PathBuf,
    pub status: Option<String>,
    pub based_on: Vec<PathBuf>,
    pub open_questions: bool,
}

pub fn read(path: &Path) -> Result<Document> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| Error::NotFound(format!("document {}", path.display())))?;
    Ok(parse(path, &text))
}

pub fn parse(path: &Path, text: &str) -> Document {
    let base = path.parent().unwrap_or(Path::new(""));
    let mut status = None;
    let mut based_on = Vec::new();
    for line in text.lines().take_while(|line| !line.starts_with("## ")) {
        if let Some(value) = line.strip_prefix("Document status:") {
            status = Some(value.trim().to_lowercase());
        }
        if let Some(value) = line.strip_prefix("Статус:") {
            status = Some(russian_status(value));
        }
        if let Some(value) = line.strip_prefix("Based on:") {
            based_on = links(value)
                .into_iter()
                .map(|target| base.join(target))
                .collect();
        }
    }
    Document {
        path: path.to_path_buf(),
        status,
        based_on,
        open_questions: has_open_questions(text),
    }
}

pub fn check_chain(starts: &[PathBuf]) -> Result<Vec<Document>> {
    let mut problems = Vec::new();
    let mut checked = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue: Vec<PathBuf> = starts.iter().rev().cloned().collect();
    while let Some(path) = queue.pop() {
        if !seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
            continue;
        }
        let Ok(document) = read(&path) else {
            problems.push(format!("{} does not exist", path.display()));
            continue;
        };
        if document.status.as_deref() != Some("approved") {
            problems.push(format!(
                "{} has status {}",
                path.display(),
                document.status.as_deref().unwrap_or("none")
            ));
        }
        if document.open_questions {
            problems.push(format!("{} has open questions", path.display()));
        }
        queue.extend(document.based_on.iter().rev().cloned());
        checked.push(document);
    }
    if !problems.is_empty() {
        return invalid(format!(
            "document chain is not approved: {}",
            problems.join("; ")
        ));
    }
    Ok(checked)
}

fn links(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].split('#').next().unwrap_or("").trim();
        if !target.is_empty() && !target.contains("://") {
            found.push(target.to_string());
        }
        rest = &after[end + 1..];
    }
    found
}

fn has_open_questions(text: &str) -> bool {
    let mut in_fence = false;
    let mut section: Option<usize> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((depth, title)) = heading(line) {
            if section.is_some_and(|open| depth <= open) {
                section = None;
            }
            if section.is_none() && is_question_heading(&title) {
                section = Some(depth);
            }
            continue;
        }
        if section.is_some() && !is_none_marker(line) {
            return true;
        }
    }
    false
}

fn heading(line: &str) -> Option<(usize, String)> {
    let depth = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&depth) {
        return None;
    }
    let title = line[depth..].strip_prefix(' ')?;
    Some((depth, title.trim().to_lowercase()))
}

fn is_question_heading(title: &str) -> bool {
    let title =
        title.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ' || c == '§');
    QUESTION_HEADINGS.contains(&title)
}

fn is_none_marker(line: &str) -> bool {
    let text = line
        .trim()
        .trim_start_matches(['-', '*'])
        .trim()
        .trim_end_matches('.')
        .to_lowercase();
    text.is_empty() || NONE_MARKERS.contains(&text.as_str())
}

fn russian_status(value: &str) -> String {
    let word = value
        .trim()
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_lowercase();
    if word.starts_with("утвержд") {
        "approved".to_string()
    } else if word.starts_with("черновик") {
        "draft".to_string()
    } else {
        word
    }
}
