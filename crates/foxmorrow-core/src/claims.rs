use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::assignment::{self, Assignment};
use crate::error::{Error, Result};
use crate::journal::Journal;
use crate::model::{Kind, VerificationState};
use crate::receipt::{self, Receipt};
use crate::report::{self, ReportBody};
use crate::run::Run;
use crate::state;
use crate::util::now_ms;

pub type ModelFamilies = BTreeMap<String, String>;

const EDIT_TOOLS: [&str; 4] = ["edit", "write", "apply_patch", "notebook_edit"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainObservation {
    pub source: String,
    pub agent_id: String,
    pub profile: String,
    pub task_text: String,
    pub system_texts: Vec<String>,
    pub models: Vec<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaimState {
    Verified,
    Failed,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub state: ClaimState,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorFacts {
    pub assignment_id: String,
    pub agent_id: String,
    pub models: Vec<String>,
}

pub struct ClaimInput<'a> {
    pub run: &'a Run,
    pub assignment: &'a Assignment,
    pub report: &'a ReportBody,
    pub receipts: &'a [Receipt],
    pub authors: &'a [AuthorFacts],
    pub families: &'a ModelFamilies,
    pub depends_accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub assignment_id: String,
    pub state: VerificationState,
    pub claims: Vec<Claim>,
}

pub fn verify(
    journal: &mut Journal,
    run: &Run,
    assignment_id: &str,
    observation: Option<ChainObservation>,
    families: &ModelFamilies,
) -> Result<Verification> {
    journal.write(|tx| {
        if state::is_accepted(tx, assignment_id)? {
            return Err(Error::Conflict(format!("{assignment_id} is already accepted")));
        }
        let claims = evaluate_recorded(tx, run, assignment_id, observation.as_ref(), families)?;
        let verdict = verdict_of(&claims);
        let observation_json = observation.as_ref().map(serde_json::to_string).transpose()?;
        tx.execute(
            "INSERT INTO verification (assignment_id, created_ms, state, claims_json, observation_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![assignment_id, now_ms(), verdict.as_str(), serde_json::to_string(&claims)?, observation_json],
        )?;
        Ok(Verification { assignment_id: assignment_id.to_string(), state: verdict, claims })
    })
}

pub fn accepted_observation(
    conn: &Connection,
    assignment_id: &str,
) -> Result<Option<ChainObservation>> {
    let raw: Option<Option<String>> = conn
        .query_row(
            "SELECT observation_json FROM verification WHERE assignment_id = ?1 AND state = 'accepted'",
            [assignment_id],
            |row| row.get(0),
        )
        .optional()?;
    match raw.flatten() {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

pub fn observed_models(conn: &Connection) -> Result<BTreeSet<String>> {
    let mut statement = conn
        .prepare("SELECT observation_json FROM verification WHERE observation_json IS NOT NULL")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut models = BTreeSet::new();
    for row in rows {
        let observed: ChainObservation = serde_json::from_str(&row?)?;
        models.extend(observed.models);
    }
    Ok(models)
}

pub(crate) fn evaluate_recorded(
    conn: &Connection,
    run: &Run,
    assignment_id: &str,
    observation: Option<&ChainObservation>,
    families: &ModelFamilies,
) -> Result<Vec<Claim>> {
    let assignment = assignment::get(conn, assignment_id)?;
    let (report, _) = report::get(conn, assignment_id)?
        .ok_or_else(|| Error::NotFound(format!("report for {assignment_id}")))?;
    let receipts = receipt::list_for(conn, assignment_id)?;
    let authors = author_facts(conn, &assignment)?;
    let mut depends_accepted = true;
    for id in &assignment.depends_on {
        depends_accepted &= state::is_accepted(conn, id)?;
    }
    let input = ClaimInput {
        run,
        assignment: &assignment,
        report: &report,
        receipts: &receipts,
        authors: &authors,
        families,
        depends_accepted,
    };
    Ok(evaluate(&input, observation))
}

pub fn evaluate(input: &ClaimInput<'_>, observation: Option<&ChainObservation>) -> Vec<Claim> {
    let order = if input.depends_accepted {
        claim("C6", ClaimState::Verified, "dependencies accepted")
    } else {
        claim("C6", ClaimState::Failed, "a dependency is not accepted")
    };
    let Some(observed) = observation else {
        let mut claims: Vec<Claim> = ["C1", "C2", "C3", "C4", "C5"]
            .iter()
            .map(|id| claim(id, ClaimState::Unverified, "no native observation"))
            .collect();
        claims.push(order);
        return claims;
    };
    vec![
        delivery(input, observed),
        authorship(input, observed),
        model(input, observed),
        actions(input, observed),
        independence(input, observed),
        order,
    ]
}

fn claim(id: &str, state: ClaimState, detail: impl Into<String>) -> Claim {
    Claim {
        id: id.to_string(),
        state,
        detail: detail.into(),
    }
}

fn verdict_of(claims: &[Claim]) -> VerificationState {
    if claims.iter().any(|claim| claim.state == ClaimState::Failed) {
        VerificationState::Rejected
    } else if claims
        .iter()
        .any(|claim| claim.state == ClaimState::Unverified)
    {
        VerificationState::Unverified
    } else {
        VerificationState::Accepted
    }
}

fn delivery(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let assignment = input.assignment;
    if assignment.kind == Kind::Root {
        return claim("C1", ClaimState::Verified, "root report without delegation");
    }
    if assignment.profile.as_deref() != Some(observed.profile.as_str()) {
        return claim(
            "C1",
            ClaimState::Failed,
            format!("profile {} does not match the assignment", observed.profile),
        );
    }
    if !has_token(&observed.task_text, &assignment.id) {
        return claim(
            "C1",
            ClaimState::Failed,
            "task does not contain the assignment id",
        );
    }
    let Some(brief) = &assignment.brief_sha256 else {
        return claim("C1", ClaimState::Unverified, "brief hash was not recorded");
    };
    if !observed.task_text.contains(brief.as_str()) {
        return claim(
            "C1",
            ClaimState::Failed,
            "task does not contain the brief hash",
        );
    }
    let Some(role) = &assignment.role_sha256 else {
        return claim("C1", ClaimState::Unverified, "role hash was not recorded");
    };
    let marker = format!("foxmorrow-role-sha256: {role}");
    if !observed
        .system_texts
        .iter()
        .any(|text| text.contains(&marker))
    {
        return claim(
            "C1",
            ClaimState::Failed,
            "role text with the recorded hash is missing",
        );
    }
    claim("C1", ClaimState::Verified, "task and role delivered")
}

fn authorship(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let id = &input.assignment.id;
    let reported = observed.tool_calls.iter().any(|call| {
        call.name == "exec"
            && call
                .command
                .as_deref()
                .is_some_and(|command| invokes(command, "report", id))
    });
    if reported {
        claim(
            "C2",
            ClaimState::Verified,
            "report submitted from this chain",
        )
    } else {
        claim(
            "C2",
            ClaimState::Failed,
            format!("no foxmorrow report call for {id} in this chain"),
        )
    }
}

fn model(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    if observed.models.is_empty() {
        return claim("C3", ClaimState::Unverified, "no model recorded");
    }
    let expected = &input.assignment.expected_models;
    if !expected.is_empty()
        && let Some(unexpected) = observed
            .models
            .iter()
            .find(|model| !expected.contains(model))
    {
        return claim(
            "C3",
            ClaimState::Failed,
            format!(
                "model {unexpected} is not expected ({})",
                expected.join(", ")
            ),
        );
    }
    if input.run.family_rule && input.assignment.kind.is_verification() {
        let Some(own) = families_of(&observed.models, input.families) else {
            return claim("C3", ClaimState::Unverified, "model family is unknown");
        };
        let author_models: Vec<String> = input
            .authors
            .iter()
            .flat_map(|author| author.models.iter().cloned())
            .collect();
        if author_models.is_empty() {
            return claim("C3", ClaimState::Unverified, "author models are unknown");
        }
        let Some(theirs) = families_of(&author_models, input.families) else {
            return claim(
                "C3",
                ClaimState::Unverified,
                "author model family is unknown",
            );
        };
        if let Some(shared) = own.intersection(&theirs).next() {
            return claim(
                "C3",
                ClaimState::Failed,
                format!("shares model family {shared} with the author"),
            );
        }
    }
    claim("C3", ClaimState::Verified, "model matches the profile")
}

fn actions(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    let id = &input.assignment.id;
    let references = report::all_evidence(input.report);
    if references.is_empty() {
        return claim("C4", ClaimState::Verified, "no evidence references");
    }
    let passing = report::passing_evidence(input.report);
    for reference in references {
        if reference.starts_with("chk-") {
            let Some(receipt) = input
                .receipts
                .iter()
                .find(|receipt| receipt.id == reference)
            else {
                return claim(
                    "C4",
                    ClaimState::Failed,
                    format!("check {reference} does not belong to {id}"),
                );
            };
            let argv = unquoted(&receipt.argv.join(" "));
            let ran_here = observed.tool_calls.iter().any(|call| {
                call.name == "exec"
                    && call.command.as_deref().is_some_and(|command| {
                        invokes(command, "check", id) && unquoted(command).contains(&argv)
                    })
            });
            if !ran_here {
                return claim(
                    "C4",
                    ClaimState::Failed,
                    format!("check {reference} was not run from this chain"),
                );
            }
        } else {
            let Some(call) = observed.tool_calls.iter().find(|call| call.id == reference) else {
                return claim(
                    "C4",
                    ClaimState::Failed,
                    format!("tool call {reference} is not in this chain"),
                );
            };
            if passing.contains(&reference)
                && let Some(code) = call.exit_code.filter(|code| *code != 0)
            {
                return claim(
                    "C4",
                    ClaimState::Failed,
                    format!("tool call {reference} exited with {code}"),
                );
            }
        }
    }
    claim("C4", ClaimState::Verified, "evidence found in this chain")
}

fn independence(input: &ClaimInput<'_>, observed: &ChainObservation) -> Claim {
    if !input.assignment.kind.is_verification() {
        return claim("C5", ClaimState::Verified, "not a verification assignment");
    }
    if let Some(author) = input
        .authors
        .iter()
        .find(|author| author.agent_id == observed.agent_id)
    {
        return claim(
            "C5",
            ClaimState::Failed,
            format!("same agent as author assignment {}", author.assignment_id),
        );
    }
    if let Some(call) = observed
        .tool_calls
        .iter()
        .find(|call| EDIT_TOOLS.contains(&call.name.as_str()))
    {
        return claim(
            "C5",
            ClaimState::Failed,
            format!("verification chain used {}", call.name),
        );
    }
    claim("C5", ClaimState::Verified, "independent of the authors")
}

fn author_facts(conn: &Connection, assignment: &Assignment) -> Result<Vec<AuthorFacts>> {
    let mut pending = assignment.depends_on.clone();
    let mut seen = BTreeSet::new();
    let mut facts = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let dependency = assignment::get(conn, &id)?;
        if dependency.kind == Kind::Author
            && let Some(observed) = accepted_observation(conn, &id)?
        {
            facts.push(AuthorFacts {
                assignment_id: id,
                agent_id: observed.agent_id,
                models: observed.models,
            });
        }
        pending.extend(dependency.depends_on);
    }
    Ok(facts)
}

fn families_of(models: &[String], families: &ModelFamilies) -> Option<BTreeSet<String>> {
    models
        .iter()
        .map(|model| families.get(model).cloned())
        .collect()
}

fn invokes(command: &str, subcommand: &str, assignment_id: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    let calls = words
        .windows(2)
        .any(|pair| is_foxmorrow(pair[0]) && pair[1] == subcommand);
    let names = words
        .windows(2)
        .any(|pair| matches!(pair[0], "-a" | "--assignment") && pair[1] == assignment_id);
    calls && names
}

fn has_token(text: &str, token: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .any(|part| part == token)
}

fn unquoted(text: &str) -> String {
    text.chars().filter(|c| *c != '"' && *c != '\'').collect()
}

fn is_foxmorrow(word: &str) -> bool {
    let name = word
        .trim_matches(|c| c == '"' || c == '\'')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    matches!(name, "foxmorrow" | "foxmorrow.exe" | "foxmorrow.cmd")
}
