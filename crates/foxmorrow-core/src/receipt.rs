use std::path::Path;
use std::process::Command;

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::assignment;
use crate::error::{Error, Result, invalid};
use crate::journal::Journal;
use crate::state;
use crate::util::{now_ms, sha256_hex};

pub const OUTPUT_TAIL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub id: String,
    pub assignment_id: String,
    pub cwd: String,
    pub argv: Vec<String>,
    pub started_ms: i64,
    pub finished_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub output_sha256: Option<String>,
    pub output_tail: Option<String>,
}

const COLUMNS: &str = "id, assignment_id, cwd, argv_json, started_ms, finished_ms, exit_code, output_sha256, output_tail";

pub fn run_check(
    journal: &mut Journal,
    assignment_id: &str,
    cwd: &Path,
    argv: &[String],
) -> Result<Receipt> {
    let Some(program) = argv.first() else {
        return invalid("command is required");
    };
    let cwd_text = cwd.display().to_string();
    let argv_json = serde_json::to_string(argv)?;
    let id = journal.write(|tx| {
        assignment::get(tx, assignment_id)?;
        if state::report_state(tx, assignment_id)?.is_some() {
            return Err(Error::Conflict(format!("{assignment_id} already has a report")));
        }
        let seq: i64 = tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM receipt", [], |row| row.get(0))?;
        let id = format!("chk-{seq:03}");
        tx.execute(
            "INSERT INTO receipt (id, seq, assignment_id, cwd, argv_json, started_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, seq, assignment_id, cwd_text, argv_json, now_ms()],
        )?;
        Ok(id)
    })?;
    let (exit_code, output) = match Command::new(program)
        .args(&argv[1..])
        .current_dir(cwd)
        .output()
    {
        Ok(result) => {
            let mut bytes = result.stdout;
            bytes.extend_from_slice(&result.stderr);
            (result.status.code(), bytes)
        }
        Err(error) => (
            None,
            format!("failed to start {program}: {error}").into_bytes(),
        ),
    };
    let tail = tail_text(&output);
    let digest = sha256_hex(&output);
    journal.write(|tx| {
        tx.execute(
            "UPDATE receipt SET finished_ms = ?1, exit_code = ?2, output_sha256 = ?3, output_tail = ?4 WHERE id = ?5",
            params![now_ms(), exit_code, digest, tail, id],
        )?;
        Ok(())
    })?;
    get(journal.conn(), &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Receipt> {
    let raw = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM receipt WHERE id = ?1"),
            [id],
            read_row,
        )
        .optional()?;
    into_receipt(raw.ok_or_else(|| Error::NotFound(format!("check {id}")))?)
}

pub fn list_for(conn: &Connection, assignment_id: &str) -> Result<Vec<Receipt>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM receipt WHERE assignment_id = ?1 ORDER BY seq"
    ))?;
    let rows = statement.query_map([assignment_id], read_row)?;
    let mut receipts = Vec::new();
    for row in rows {
        receipts.push(into_receipt(row?)?);
    }
    Ok(receipts)
}

fn tail_text(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(OUTPUT_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<(Receipt, String)> {
    Ok((
        Receipt {
            id: row.get(0)?,
            assignment_id: row.get(1)?,
            cwd: row.get(2)?,
            argv: Vec::new(),
            started_ms: row.get(4)?,
            finished_ms: row.get(5)?,
            exit_code: row.get(6)?,
            output_sha256: row.get(7)?,
            output_tail: row.get(8)?,
        },
        row.get(3)?,
    ))
}

fn into_receipt((receipt, argv_json): (Receipt, String)) -> Result<Receipt> {
    Ok(Receipt {
        argv: serde_json::from_str(&argv_json)?,
        ..receipt
    })
}
