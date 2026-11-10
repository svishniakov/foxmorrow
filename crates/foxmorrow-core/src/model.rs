use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Author,
    Review,
    Qa,
    Root,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Completed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Passed,
    Failed,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationState {
    Accepted,
    Rejected,
    Unverified,
}

fn parse_named<T: Copy>(
    value: &str,
    all: &[T],
    name: fn(T) -> &'static str,
    what: &str,
) -> Result<T> {
    match all
        .iter()
        .copied()
        .find(|candidate| name(*candidate) == value)
    {
        Some(found) => Ok(found),
        None => invalid(format!("unknown {what} '{value}'")),
    }
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Author => "author",
            Kind::Review => "review",
            Kind::Qa => "qa",
            Kind::Root => "root",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(
            value,
            &[Kind::Author, Kind::Review, Kind::Qa, Kind::Root],
            Self::as_str,
            "assignment kind",
        )
    }

    pub fn is_verification(self) -> bool {
        matches!(self, Kind::Review | Kind::Qa)
    }
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Completed => "completed",
            Status::Interrupted => "interrupted",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(
            value,
            &[Status::Completed, Status::Interrupted],
            Self::as_str,
            "report status",
        )
    }
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Passed => "passed",
            Outcome::Failed => "failed",
            Outcome::Unverified => "unverified",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(
            value,
            &[Outcome::Passed, Outcome::Failed, Outcome::Unverified],
            Self::as_str,
            "outcome",
        )
    }
}

impl VerificationState {
    pub fn as_str(self) -> &'static str {
        match self {
            VerificationState::Accepted => "accepted",
            VerificationState::Rejected => "rejected",
            VerificationState::Unverified => "unverified",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        parse_named(
            value,
            &[
                VerificationState::Accepted,
                VerificationState::Rejected,
                VerificationState::Unverified,
            ],
            Self::as_str,
            "verification state",
        )
    }
}
