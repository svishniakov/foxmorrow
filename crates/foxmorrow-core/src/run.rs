use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::baseline::RepoBaseline;
use crate::criteria::{self, Criterion};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::util::{new_run_id, now_ms, sha256_hex};

#[derive(Debug, Clone)]
pub struct NewRun {
    pub project_root: PathBuf,
    pub criteria_path: String,
    pub criteria_heading: String,
    pub criteria_markdown: String,
    pub allowed_paths: Vec<String>,
    pub baselines: Vec<RepoBaseline>,
    pub root_session_id: Option<String>,
    pub family_rule: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub id: String,
    pub created_ms: i64,
    pub project_root: String,
    pub criteria_path: String,
    pub criteria_heading: String,
    pub criteria_sha256: String,
    pub criteria: Vec<Criterion>,
    pub allowed_paths: Vec<String>,
    pub family_rule: bool,
    pub root_session_id: Option<String>,
    pub root_model: Option<String>,
}

pub fn foxmorrow_dir(project_root: &Path) -> PathBuf {
    project_root.join(".foxmorrow")
}

pub fn runs_dir(project_root: &Path) -> PathBuf {
    foxmorrow_dir(project_root).join("runs")
}

pub fn create_run(new: NewRun) -> Result<(Run, Journal)> {
    if new.allowed_paths.is_empty() {
        return invalid("at least one allowed path is required");
    }
    let (section, criteria) =
        criteria::parse_section(&new.criteria_markdown, &new.criteria_heading)?;
    let id = new_run_id();
    let mut journal = Journal::create(&runs_dir(&new.project_root).join(&id))?;
    let run = Run {
        id: id.clone(),
        created_ms: now_ms(),
        project_root: new.project_root.display().to_string(),
        criteria_path: new.criteria_path,
        criteria_heading: new.criteria_heading,
        criteria_sha256: sha256_hex(section.as_bytes()),
        criteria,
        allowed_paths: new.allowed_paths,
        family_rule: new.family_rule,
        root_session_id: new.root_session_id,
        root_model: None,
    };
    let baseline_json = serde_json::to_string(&new.baselines)?;
    journal.write(|tx| {
        tx.execute(
            "INSERT INTO run (id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, baseline_json, root_session_id, root_model, family_rule)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                run.id,
                run.created_ms,
                run.project_root,
                run.criteria_path,
                run.criteria_heading,
                run.criteria_sha256,
                serde_json::to_string(&run.criteria)?,
                serde_json::to_string(&run.allowed_paths)?,
                baseline_json,
                run.root_session_id,
                run.root_model,
                run.family_rule
            ],
        )?;
        Ok(())
    })?;
    std::fs::write(
        foxmorrow_dir(&new.project_root).join("current"),
        format!("{id}\n"),
    )?;
    Ok((run, journal))
}

pub fn current_run_id(project_root: &Path) -> Result<String> {
    let path = foxmorrow_dir(project_root).join("current");
    let missing = || Error::NotFound("no current run; start one with foxmorrow init".into());
    let text = std::fs::read_to_string(&path).map_err(|_| missing())?;
    let id = text.trim();
    if id.is_empty() {
        return Err(missing());
    }
    Ok(id.to_string())
}

pub fn open_run(project_root: &Path, run_id: Option<&str>) -> Result<(Run, Journal)> {
    let id = match run_id {
        Some(id) => id.to_string(),
        None => current_run_id(project_root)?,
    };
    let journal = Journal::open(&runs_dir(project_root).join(&id))?;
    let run = load_run(journal.conn())?;
    Ok((run, journal))
}

pub fn load_run(conn: &Connection) -> Result<Run> {
    let (run, criteria_json, paths_json) = conn.query_row(
        "SELECT id, created_ms, project_root, criteria_path, criteria_heading, criteria_sha256, criteria_json, allowed_paths_json, family_rule, root_session_id, root_model FROM run",
        [],
        |row| {
            Ok((
                Run {
                    id: row.get(0)?,
                    created_ms: row.get(1)?,
                    project_root: row.get(2)?,
                    criteria_path: row.get(3)?,
                    criteria_heading: row.get(4)?,
                    criteria_sha256: row.get(5)?,
                    criteria: Vec::new(),
                    allowed_paths: Vec::new(),
                    family_rule: row.get(8)?,
                    root_session_id: row.get(9)?,
                    root_model: row.get(10)?,
                },
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        },
    )?;
    Ok(Run {
        criteria: serde_json::from_str(&criteria_json)?,
        allowed_paths: serde_json::from_str(&paths_json)?,
        ..run
    })
}
