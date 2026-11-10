use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::run::Run;
use crate::state;
use crate::util::{now_ms, sha256_hex};

#[derive(Debug, Clone)]
pub struct NewAssignment {
    pub role: String,
    pub kind: Kind,
    pub profile: Option<String>,
    pub expected_models: Vec<String>,
    pub brief_sha256: Option<String>,
    pub role_sha256: Option<String>,
    pub allowed_paths: Vec<String>,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assignment {
    pub id: String,
    pub seq: i64,
    pub created_ms: i64,
    pub role: String,
    pub kind: Kind,
    pub profile: Option<String>,
    pub expected_models: Vec<String>,
    pub brief_sha256: Option<String>,
    pub role_sha256: Option<String>,
    pub allowed_paths: Vec<String>,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
}

const COLUMNS: &str = "id, seq, created_ms, role, kind, profile, expected_models_json, brief_sha256, role_sha256, allowed_paths_json, depends_on_json, previous_id";

type RawAssignment = (Assignment, String, String, String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefDraft {
    pub language: String,
    pub body: String,
}

pub fn create(journal: &mut Journal, run: &Run, new: NewAssignment) -> Result<Assignment> {
    Ok(insert(journal, run, new, |_| Ok(None))?.0)
}

pub fn create_with_brief(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    render: impl FnOnce(&Assignment) -> Result<BriefDraft>,
) -> Result<(Assignment, String)> {
    let (created, text) = insert(journal, run, new, |draft| render(draft).map(Some))?;
    let text = text.ok_or_else(|| Error::Invalid("brief was not rendered".into()))?;
    Ok((created, text))
}

pub fn brief(conn: &Connection, id: &str) -> Result<Option<(String, String)>> {
    Ok(conn
        .query_row(
            "SELECT language, text FROM brief WHERE assignment_id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

fn insert(
    journal: &mut Journal,
    run: &Run,
    new: NewAssignment,
    render: impl FnOnce(&Assignment) -> Result<Option<BriefDraft>>,
) -> Result<(Assignment, Option<String>)> {
    validate_shape(run, &new)?;
    journal.write(|tx| {
        validate_links(tx, &new)?;
        let seq: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM assignment", [], |row| row.get(0))?;
        let mut created = Assignment {
            id: format!("a-{seq:02}"),
            seq,
            created_ms: now_ms(),
            role: new.role,
            kind: new.kind,
            profile: new.profile,
            expected_models: new.expected_models,
            brief_sha256: new.brief_sha256,
            role_sha256: new.role_sha256,
            allowed_paths: new.allowed_paths,
            depends_on: new.depends_on,
            previous_id: new.previous_id,
        };
        let brief = match render(&created)? {
            Some(draft) => {
                let digest = sha256_hex(draft.body.as_bytes());
                let text = format!(
                    "foxmorrow assignment {} run {} brief-sha256 {digest}\n\n{}",
                    created.id, run.id, draft.body
                );
                created.brief_sha256 = Some(digest);
                Some((draft.language, text))
            }
            None => None,
        };
        tx.execute(
            &format!("INSERT INTO assignment ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
            params![
                created.id,
                created.seq,
                created.created_ms,
                created.role,
                created.kind.as_str(),
                created.profile,
                serde_json::to_string(&created.expected_models)?,
                created.brief_sha256,
                created.role_sha256,
                serde_json::to_string(&created.allowed_paths)?,
                serde_json::to_string(&created.depends_on)?,
                created.previous_id
            ],
        )?;
        if let Some((language, text)) = &brief {
            tx.execute(
                "INSERT INTO brief (assignment_id, language, text) VALUES (?1, ?2, ?3)",
                params![created.id, language, text],
            )?;
        }
        Ok((created, brief.map(|(_, text)| text)))
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Assignment> {
    let raw = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM assignment WHERE id = ?1"),
            [id],
            read_row,
        )
        .optional()?;
    into_assignment(raw.ok_or_else(|| Error::NotFound(format!("assignment {id}")))?)
}

pub fn list(conn: &Connection) -> Result<Vec<Assignment>> {
    let mut statement = conn.prepare(&format!("SELECT {COLUMNS} FROM assignment ORDER BY seq"))?;
    let rows = statement.query_map([], read_row)?;
    let mut assignments = Vec::new();
    for row in rows {
        assignments.push(into_assignment(row?)?);
    }
    Ok(assignments)
}

pub fn within(path: &str, roots: &[String]) -> bool {
    let path = normalize(path);
    if path.split('/').any(|part| part == "..") {
        return false;
    }
    roots
        .iter()
        .map(|root| normalize(root))
        .any(|root| root.is_empty() || path == root || path.starts_with(&format!("{root}/")))
}

fn normalize(path: &str) -> String {
    let trimmed = path.trim();
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    if trimmed == "." {
        String::new()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn validate_shape(run: &Run, new: &NewAssignment) -> Result<()> {
    if new.role.trim().is_empty() {
        return invalid("role is required");
    }
    if new.allowed_paths.is_empty() {
        return invalid("at least one allowed path is required");
    }
    if let Some(outside) = new
        .allowed_paths
        .iter()
        .find(|path| !within(path, &run.allowed_paths))
    {
        return invalid(format!("path '{outside}' is outside the run allowed paths"));
    }
    if new.kind != Kind::Root {
        if new
            .profile
            .as_deref()
            .is_none_or(|profile| profile.trim().is_empty())
        {
            return invalid("profile is required");
        }
        if new.expected_models.is_empty() {
            return invalid("expected model is required");
        }
    }
    Ok(())
}

fn validate_links(conn: &Connection, new: &NewAssignment) -> Result<()> {
    if let Some(previous) = &new.previous_id {
        let corrected = get(conn, previous)?;
        if corrected.kind != new.kind {
            return invalid(format!(
                "correction of {previous} must keep kind {}, got {}",
                corrected.kind.as_str(),
                new.kind.as_str()
            ));
        }
        if state::report_state(conn, previous)?.is_none() {
            return invalid(format!("{previous} has no report to correct"));
        }
    }
    let mut dependencies = Vec::new();
    for id in &new.depends_on {
        dependencies.push(get(conn, id)?);
    }
    match new.kind {
        Kind::Root => {}
        Kind::Author => {
            for dependency in &dependencies {
                if !state::is_accepted(conn, &dependency.id)? {
                    return invalid(format!("dependency {} is not accepted", dependency.id));
                }
            }
        }
        Kind::Review => {
            if dependencies.is_empty() {
                return invalid("review requires author dependencies");
            }
            for dependency in &dependencies {
                if dependency.kind != Kind::Author {
                    return invalid(format!(
                        "review dependency {} is not an author assignment",
                        dependency.id
                    ));
                }
                require_completed_and_accepted(conn, &dependency.id)?;
            }
        }
        Kind::Qa => {
            let mut passed_review = false;
            for dependency in &dependencies {
                match dependency.kind {
                    Kind::Review => {
                        let outcome = require_completed_and_accepted(conn, &dependency.id)?;
                        if outcome != Outcome::Passed {
                            return invalid(format!("review {} did not pass", dependency.id));
                        }
                        passed_review = true;
                    }
                    Kind::Author => {
                        require_completed_and_accepted(conn, &dependency.id)?;
                    }
                    _ => {
                        return invalid(format!(
                            "qa dependency {} must be an author or review",
                            dependency.id
                        ));
                    }
                }
            }
            if !passed_review {
                return invalid("qa requires a passed review dependency");
            }
        }
    }
    Ok(())
}

fn require_completed_and_accepted(conn: &Connection, id: &str) -> Result<Outcome> {
    let Some((status, outcome)) = state::report_state(conn, id)? else {
        return invalid(format!("dependency {id} has no report"));
    };
    if status != Status::Completed {
        return invalid(format!("dependency {id} is not completed"));
    }
    if !state::is_accepted(conn, id)? {
        return invalid(format!("dependency {id} is not accepted"));
    }
    Ok(outcome)
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<RawAssignment> {
    Ok((
        Assignment {
            id: row.get(0)?,
            seq: row.get(1)?,
            created_ms: row.get(2)?,
            role: row.get(3)?,
            kind: Kind::Root,
            profile: row.get(5)?,
            expected_models: Vec::new(),
            brief_sha256: row.get(7)?,
            role_sha256: row.get(8)?,
            allowed_paths: Vec::new(),
            depends_on: Vec::new(),
            previous_id: row.get(11)?,
        },
        row.get(4)?,
        row.get(6)?,
        row.get(9)?,
        row.get(10)?,
    ))
}

fn into_assignment(
    (assignment, kind, models, paths, depends): RawAssignment,
) -> Result<Assignment> {
    Ok(Assignment {
        kind: Kind::parse(&kind)?,
        expected_models: serde_json::from_str(&models)?,
        allowed_paths: serde_json::from_str(&paths)?,
        depends_on: serde_json::from_str(&depends)?,
        ..assignment
    })
}
