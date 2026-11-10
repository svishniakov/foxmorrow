mod project;

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use foxmorrow_core::assignment::{self, NewAssignment};
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::{Result, baseline, criteria, receipt, report, status};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "foxmorrow", version, about = "Provable multi-agent runs")]
struct Cli {
    #[arg(long, global = true)]
    project: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        criteria: String,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "repo")]
        repos: Vec<PathBuf>,
        #[arg(long)]
        no_family_rule: bool,
        #[arg(long)]
        session_id: Option<String>,
    },
    Assign {
        #[arg(long)]
        role: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long = "expected-model")]
        expected_models: Vec<String>,
        #[arg(long)]
        brief_sha256: Option<String>,
        #[arg(long)]
        role_sha256: Option<String>,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "depends-on", num_args = 1..)]
        depends_on: Vec<String>,
        #[arg(long)]
        previous: Option<String>,
        #[arg(long)]
        run: Option<String>,
    },
    Check {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    Report {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
    },
    Status {
        #[arg(long)]
        run: Option<String>,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("foxmorrow: error: {error}");
            ExitCode::from(2)
        }
    }
}

fn execute(cli: Cli) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    match cli.command {
        Command::Init {
            criteria: reference,
            paths,
            repos,
            no_family_rule,
            session_id,
        } => {
            let root = project::for_init(cli.project, &cwd);
            let (file, heading) = criteria::split_reference(&reference)?;
            let markdown = std::fs::read_to_string(root.join(file))?;
            let repos = if repos.is_empty() {
                vec![root.clone()]
            } else {
                repos
            };
            let mut baselines = Vec::new();
            for repo in &repos {
                baselines.push(baseline::collect(repo)?);
            }
            let (created, _) = run::create_run(NewRun {
                project_root: root,
                criteria_path: file.to_string(),
                criteria_heading: heading.to_string(),
                criteria_markdown: markdown,
                allowed_paths: paths,
                baselines,
                root_session_id: session_id,
                family_rule: !no_family_rule,
            })?;
            print_json(&serde_json::json!({ "run": created.id, "criteria": created.criteria }))?;
        }
        Command::Assign {
            role,
            kind,
            profile,
            expected_models,
            brief_sha256,
            role_sha256,
            paths,
            depends_on,
            previous,
            run: run_id,
        } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let created = assignment::create(
                &mut journal,
                &current,
                NewAssignment {
                    role,
                    kind: Kind::parse(&kind)?,
                    profile,
                    expected_models,
                    brief_sha256,
                    role_sha256,
                    allowed_paths: paths,
                    depends_on,
                    previous_id: previous,
                },
            )?;
            print_json(&created)?;
        }
        Command::Check {
            assignment: id,
            run: run_id,
            command,
        } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (_, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let recorded = receipt::run_check(&mut journal, &id, &cwd, &command)?;
            print!("{}", recorded.output_tail.as_deref().unwrap_or(""));
            let shown = recorded
                .exit_code
                .map_or_else(|| "none".to_string(), |code| code.to_string());
            eprintln!("foxmorrow: {} exit={shown}", recorded.id);
            let code = recorded
                .exit_code
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1);
            return Ok(ExitCode::from(code));
        }
        Command::Report {
            assignment: id,
            run: run_id,
        } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&root, run_id.as_deref())?;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let body = report::submit(&mut journal, &current, &id, &input)?;
            print_json(
                &serde_json::json!({ "assignment": id, "status": body.status, "outcome": body.outcome }),
            )?;
        }
        Command::Status { run: run_id } => {
            let root = project::for_existing(cli.project, &cwd)?;
            let (current, journal) = run::open_run(&root, run_id.as_deref())?;
            print_json(&status::status(journal.conn(), &current)?)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
