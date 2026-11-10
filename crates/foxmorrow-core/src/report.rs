use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::assignment::{self, Assignment};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::receipt::{self, Receipt};
use crate::run::Run;
use crate::util::{now_ms, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CriterionResult {
    pub status: Outcome,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportBody {
    pub summary: String,
    pub status: Status,
    pub outcome: Outcome,
    #[serde(default)]
    pub criteria: BTreeMap<String, CriterionResult>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub resolves: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unverified: Vec<String>,
}

pub fn submit(
    journal: &mut Journal,
    run: &Run,
    assignment_id: &str,
    input: &str,
) -> Result<ReportBody> {
    let body: ReportBody = serde_json::from_str(input)?;
    journal.write(|tx| {
        let assignment = assignment::get(tx, assignment_id)?;
        if get(tx, assignment_id)?.is_some() {
            return Err(Error::Conflict(format!(
                "{assignment_id} already has a report; create a correction with --previous {assignment_id}"
            )));
        }
        let receipts = receipt::list_for(tx, assignment_id)?;
        validate(tx, run, &assignment, &receipts, &body)?;
        let json = serde_json::to_string(&body)?;
        tx.execute(
            "INSERT INTO report (assignment_id, created_ms, status, outcome, body_json, body_sha256) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![assignment_id, now_ms(), body.status.as_str(), body.outcome.as_str(), json, sha256_hex(json.as_bytes())],
        )?;
        Ok(())
    })?;
    Ok(body)
}

pub fn get(conn: &Connection, assignment_id: &str) -> Result<Option<(ReportBody, String)>> {
    let raw: Option<(String, String)> = conn
        .query_row(
            "SELECT body_json, body_sha256 FROM report WHERE assignment_id = ?1",
            [assignment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match raw {
        Some((json, digest)) => Ok(Some((serde_json::from_str(&json)?, digest))),
        None => Ok(None),
    }
}

pub fn all_evidence(body: &ReportBody) -> Vec<&str> {
    body.evidence
        .iter()
        .chain(
            body.criteria
                .values()
                .flat_map(|result| result.evidence.iter()),
        )
        .map(String::as_str)
        .collect()
}

pub fn passing_evidence(body: &ReportBody) -> Vec<&str> {
    let mut references: Vec<&str> = body
        .criteria
        .values()
        .filter(|result| result.status == Outcome::Passed)
        .flat_map(|result| result.evidence.iter().map(String::as_str))
        .collect();
    if body.outcome == Outcome::Passed {
        references.extend(body.evidence.iter().map(String::as_str));
    }
    references
}

fn validate(
    conn: &Connection,
    run: &Run,
    assignment: &Assignment,
    receipts: &[Receipt],
    body: &ReportBody,
) -> Result<()> {
    if body.summary.trim().is_empty() {
        return invalid("summary must not be empty");
    }
    if body.outcome == Outcome::Passed && body.status != Status::Completed {
        return invalid("outcome passed requires status completed");
    }
    if let Some(open) = receipts
        .iter()
        .find(|receipt| receipt.finished_ms.is_none())
        && body.status != Status::Interrupted
    {
        return invalid(format!(
            "check {} has no recorded result; inspect it and report status interrupted",
            open.id
        ));
    }
    let known: BTreeSet<&str> = run
        .criteria
        .iter()
        .map(|criterion| criterion.id.as_str())
        .collect();
    for (id, result) in &body.criteria {
        if !known.contains(id.as_str()) {
            return invalid(format!("unknown criterion {id}"));
        }
        if result.reason.trim().is_empty() {
            return invalid(format!("criterion {id} needs a reason"));
        }
    }
    if assignment.kind == Kind::Qa {
        if let Some(missing) = run
            .criteria
            .iter()
            .find(|criterion| !body.criteria.contains_key(&criterion.id))
        {
            return invalid(format!("qa report must cover {}", missing.id));
        }
        if body.outcome == Outcome::Passed
            && body
                .criteria
                .values()
                .any(|result| result.status != Outcome::Passed)
        {
            return invalid("qa outcome passed requires every criterion passed");
        }
    }
    let passing = passing_evidence(body);
    for reference in all_evidence(body)
        .into_iter()
        .filter(|reference| reference.starts_with("chk-"))
    {
        let Some(receipt) = receipts.iter().find(|receipt| receipt.id == reference) else {
            return invalid(format!(
                "check {reference} does not belong to {}",
                assignment.id
            ));
        };
        if receipt.finished_ms.is_none() {
            return invalid(format!("check {reference} has no recorded result"));
        }
        if passing.contains(&reference) && receipt.exit_code != Some(0) {
            return invalid(format!(
                "check {reference} did not succeed (exit code {:?})",
                receipt.exit_code
            ));
        }
    }
    for previous in body.resolves.keys() {
        match get(conn, previous)? {
            None => return invalid(format!("resolved assignment {previous} has no report")),
            Some((resolved, _)) if resolved.outcome == Outcome::Passed => {
                return invalid(format!("{previous} already passed; nothing to resolve"));
            }
            Some(_) => {}
        }
    }
    if assignment.kind == Kind::Root && body.outcome != Outcome::Passed {
        if body
            .reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return invalid("root stop report requires a reason");
        }
        if body.outcome == Outcome::Unverified && body.unverified.is_empty() {
            return invalid("root stop report requires the unverified list");
        }
    }
    Ok(())
}
