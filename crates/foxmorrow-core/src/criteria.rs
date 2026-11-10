use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, invalid};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub text: String,
}

pub fn split_reference(reference: &str) -> Result<(&str, &str)> {
    match reference.split_once('#') {
        Some((path, heading)) if !path.is_empty() && !heading.is_empty() => Ok((path, heading)),
        _ => invalid(format!(
            "criteria reference '{reference}' must look like <file>#<heading>"
        )),
    }
}

pub fn parse_section(markdown: &str, heading: &str) -> Result<(String, Vec<Criterion>)> {
    let wanted = heading.trim().to_lowercase();
    let mut in_fence = false;
    let mut found = false;
    let mut started = false;
    let mut section = Vec::new();
    let mut texts = Vec::new();
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(title) = heading_text(line) {
            if started {
                break;
            }
            started = title.to_lowercase() == wanted;
            found |= started;
            continue;
        } else if started
            && !in_fence
            && let Some(text) = bullet_text(line)
        {
            texts.push(text);
        }
        if started {
            section.push(line);
        }
    }
    if !found {
        return Err(Error::NotFound(format!("heading '{heading}'")));
    }
    if texts.is_empty() {
        return invalid(format!("section '{heading}' has no top-level list items"));
    }
    let criteria = texts
        .into_iter()
        .enumerate()
        .map(|(index, text)| Criterion {
            id: format!("ac-{}", index + 1),
            text,
        })
        .collect();
    Ok((section.join("\n"), criteria))
}

fn heading_text(line: &str) -> Option<String> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let title = line[hashes..].strip_prefix(' ')?;
    Some(title.trim().trim_end_matches('#').trim().to_string())
}

fn bullet_text(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| numbered(line))?;
    let text = rest.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn numbered(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    line[digits..].strip_prefix(". ")
}
