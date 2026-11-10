use rusqlite::{Connection, OptionalExtension};

use crate::error::Result;
use crate::model::{Outcome, Status, VerificationState};

pub fn report_state(conn: &Connection, assignment_id: &str) -> Result<Option<(Status, Outcome)>> {
    let raw: Option<(String, String)> = conn
        .query_row(
            "SELECT status, outcome FROM report WHERE assignment_id = ?1",
            [assignment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match raw {
        Some((status, outcome)) => Ok(Some((Status::parse(&status)?, Outcome::parse(&outcome)?))),
        None => Ok(None),
    }
}

pub fn is_accepted(conn: &Connection, assignment_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM verification WHERE assignment_id = ?1 AND state = 'accepted')",
        [assignment_id],
        |row| row.get(0),
    )?)
}

pub fn latest_verification(
    conn: &Connection,
    assignment_id: &str,
) -> Result<Option<VerificationState>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT state FROM verification WHERE assignment_id = ?1 ORDER BY id DESC LIMIT 1",
            [assignment_id],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(|state| VerificationState::parse(&state))
        .transpose()
}

pub fn is_finalized(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row("SELECT EXISTS (SELECT 1 FROM final)", [], |row| row.get(0))?)
}
