use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::assignment;
use crate::criteria::Criterion;
use crate::error::Result;
use crate::model::{Kind, Outcome, Status, VerificationState};
use crate::receipt;
use crate::run::Run;
use crate::state;

#[derive(Debug, Serialize)]
pub struct StatusView {
    pub run_id: String,
    pub criteria: Vec<Criterion>,
    pub family_rule: bool,
    pub root_model: Option<String>,
    pub assignments: Vec<AssignmentStatus>,
    pub final_outcome: Option<Outcome>,
}

#[derive(Debug, Serialize)]
pub struct AssignmentStatus {
    pub id: String,
    pub role: String,
    pub kind: Kind,
    pub depends_on: Vec<String>,
    pub previous_id: Option<String>,
    pub report: Option<ReportState>,
    pub verification: Option<VerificationState>,
    pub open_checks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReportState {
    pub status: Status,
    pub outcome: Outcome,
}

pub fn status(conn: &Connection, run: &Run) -> Result<StatusView> {
    let mut assignments = Vec::new();
    for entry in assignment::list(conn)? {
        let report = state::report_state(conn, &entry.id)?
            .map(|(status, outcome)| ReportState { status, outcome });
        let verification = state::latest_verification(conn, &entry.id)?;
        let open_checks = receipt::list_for(conn, &entry.id)?
            .into_iter()
            .filter(|receipt| receipt.finished_ms.is_none())
            .map(|receipt| receipt.id)
            .collect();
        assignments.push(AssignmentStatus {
            id: entry.id,
            role: entry.role,
            kind: entry.kind,
            depends_on: entry.depends_on,
            previous_id: entry.previous_id,
            report,
            verification,
            open_checks,
        });
    }
    let final_outcome: Option<String> = conn
        .query_row("SELECT outcome FROM final", [], |row| row.get(0))
        .optional()?;
    Ok(StatusView {
        run_id: run.id.clone(),
        criteria: run.criteria.clone(),
        family_rule: run.family_rule,
        root_model: run.root_model.clone(),
        assignments,
        final_outcome: final_outcome
            .map(|outcome| Outcome::parse(&outcome))
            .transpose()?,
    })
}
