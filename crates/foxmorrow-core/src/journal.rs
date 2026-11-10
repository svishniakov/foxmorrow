use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::error::{Error, Result, invalid};
use crate::schema::{CONTRACT_VERSION, DDL};

pub const JOURNAL_FILE: &str = "journal.sqlite";

pub struct Journal {
    conn: Connection,
    dir: PathBuf,
}

impl Journal {
    pub fn create(dir: &Path) -> Result<Self> {
        let path = dir.join(JOURNAL_FILE);
        if path.exists() {
            return Err(Error::Conflict(format!(
                "journal already exists at {}",
                path.display()
            )));
        }
        std::fs::create_dir_all(dir)?;
        let conn = connect(&path)?;
        conn.execute_batch(DDL)?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('contract_version', ?1)",
            params![CONTRACT_VERSION],
        )?;
        Ok(Self {
            conn,
            dir: dir.to_path_buf(),
        })
    }

    pub fn open(dir: &Path) -> Result<Self> {
        let path = dir.join(JOURNAL_FILE);
        if !path.exists() {
            return Err(Error::NotFound(format!("journal at {}", path.display())));
        }
        let conn = connect(&path)?;
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'contract_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if version.as_deref() != Some(CONTRACT_VERSION) {
            return invalid(format!(
                "unsupported journal contract {version:?}; this foxmorrow reads contract {CONTRACT_VERSION}"
            ));
        }
        Ok(Self {
            conn,
            dir: dir.to_path_buf(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn write<T>(&mut self, apply: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = apply(&tx)?;
        tx.commit()?;
        Ok(value)
    }
}

fn connect(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(conn)
}
