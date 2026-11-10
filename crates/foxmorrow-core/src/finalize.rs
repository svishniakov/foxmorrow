use std::collections::BTreeSet;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::assignment;
use crate::claims::{self, ClaimState, ModelFamilies};
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::model::{Kind, Outcome, Status};
use crate::report;
use crate::run::Run;
use crate::state;
use crate::util::now_ms;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalRecord {
    pub run_id: String,
    pub report_id: String,
    pub outcome: Outcome,
    pub family_rule: bool,
    pub root_model: Option<String>,
    pub reports: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unverified: Vec<String>,
}

pub fn finalize(
    journal: &mut Journal,
    run: &Run,
    report_id: &str,
    families: &ModelFamilies,
) -> Result<FinalRecord> {
    journal.write(|tx| {
        if state::is_finalized(tx)? {
            return Err(Error::Conflict("run is already finalized".into()));
        }
        let selected = assignment::get(tx, report_id)?;
        let Some((body, _)) = report::get(tx, report_id)? else {
            return invalid(format!("{report_id} has no report"));
        };
        match selected.kind {
            Kind::Qa if body.status == Status::Completed && body.outcome == Outcome::Passed => {}
            Kind::Qa => return invalid("a non-positive final requires a root stop report"),
            Kind::Root if body.outcome != Outcome::Passed => {}
            Kind::Root => return invalid("a root report can only stop the run with a non-positive outcome"),
            _ => return invalid("final requires a qa or root report"),
        }
        let reports = dependency_closure(tx, report_id)?;
        for id in &reports {
            let Some(observed) = claims::accepted_observation(tx, id)? else {
                return invalid(format!("{id} is not accepted"));
            };
            let recheck = claims::evaluate_recorded(tx, run, id, Some(&observed), families)?;
            if let Some(failed) = recheck.iter().find(|claim| claim.state != ClaimState::Verified) {
                return invalid(format!(
                    "{id} no longer verifies from its snapshot: {} {}",
                    failed.id, failed.detail
                ));
            }
        }
        let record = FinalRecord {
            run_id: run.id.clone(),
            report_id: report_id.to_string(),
            outcome: body.outcome,
            family_rule: run.family_rule,
            root_model: run.root_model.clone(),
            reports,
            reason: body.reason.clone(),
            unverified: body.unverified.clone(),
        };
        tx.execute(
            "INSERT INTO final (run_id, created_ms, report_id, outcome, body_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.run_id, now_ms(), record.report_id, record.outcome.as_str(), serde_json::to_string(&record)?],
        )?;
        Ok(record)
    })
}

fn dependency_closure(conn: &Connection, start: &str) -> Result<Vec<String>> {
    let mut pending = vec![start.to_string()];
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let current = assignment::get(conn, &id)?;
        ordered.push((current.seq, id));
        pending.extend(current.depends_on);
    }
    ordered.sort();
    Ok(ordered.into_iter().map(|(_, id)| id).collect())
}
