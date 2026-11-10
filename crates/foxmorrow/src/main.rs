mod project;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use foxmorrow_core::assignment::{self, BriefDraft, NewAssignment};
use foxmorrow_core::brief::{self, BriefInput, ContextItem};
use foxmorrow_core::catalog::Catalog;
use foxmorrow_core::error::invalid;
use foxmorrow_core::model::Kind;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::{Error, Result, baseline, criteria, receipt, report, status};
use foxmorrow_devin::agents::{self, PROJECT_PREFIX};
use foxmorrow_devin::profile::{Profile, ProjectProfile};
use foxmorrow_devin::skills::{self, Inventory};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "foxmorrow", version, about = "Provable multi-agent runs")]
struct Cli {
    #[arg(long, global = true)]
    project: Option<PathBuf>,
    #[arg(long, global = true)]
    plugin_root: Option<PathBuf>,
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
        session_id: Option<String>,
    },
    Assign {
        #[arg(long, required_unless_present = "root_assignment")]
        role: Option<String>,
        #[arg(long = "root", conflicts_with = "role")]
        root_assignment: bool,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long = "paths", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long = "depends-on", num_args = 1..)]
        depends_on: Vec<String>,
        #[arg(long)]
        previous: Option<String>,
        #[arg(long)]
        allow_missing_skills: bool,
        #[arg(long)]
        run: Option<String>,
    },
    Brief {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
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
    Sync {
        #[arg(long)]
        plugin: bool,
        #[arg(long)]
        check: bool,
    },
    Doctor,
}

struct Plugin {
    root: PathBuf,
    catalog: Catalog,
    profile: Profile,
}

impl Plugin {
    fn load(explicit: Option<PathBuf>) -> Result<Self> {
        let root = project::plugin_root(explicit)?;
        Ok(Self {
            catalog: Catalog::load(&root)?,
            profile: Profile::load(&root)?,
            root,
        })
    }
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
            session_id,
        } => {
            let root = project::for_init(cli.project, &cwd);
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_profile = ProjectProfile::load(&root)?;
            let found = inventory(&root);
            let skills_summary = missing_required(&plugin.catalog, &found);
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
                family_rule: plugin.profile.family_rule(&project_profile),
                skills_summary: skills_summary.clone(),
            })?;
            print_json(&serde_json::json!({
                "run": created.id,
                "criteria": created.criteria,
                "family_rule": created.family_rule,
                "skills_source": found.source,
                "missing_skills": skills_summary,
            }))?;
        }
        Command::Assign {
            role,
            root_assignment,
            goal,
            paths,
            depends_on,
            previous,
            allow_missing_skills,
            run: run_id,
        } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            if root_assignment {
                let created = assignment::create(
                    &mut journal,
                    &current,
                    NewAssignment {
                        role: "root".into(),
                        kind: Kind::Root,
                        profile: None,
                        expected_models: Vec::new(),
                        brief_sha256: None,
                        role_sha256: None,
                        allowed_paths: paths,
                        depends_on,
                        previous_id: previous,
                    },
                )?;
                print_json(&created)?;
                return Ok(ExitCode::SUCCESS);
            }
            let Some(role) = role else {
                return invalid("--role or --root is required");
            };
            let Some(goal) = goal else {
                return invalid("--goal is required for a delegated assignment");
            };
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_profile = ProjectProfile::load(&project_root)?;
            let spec = plugin.catalog.role(&role)?.clone();
            let agent = agents::definition(
                &plugin.root,
                &project_root,
                &plugin.catalog,
                &plugin.profile,
                &project_profile,
                &role,
            )?;
            agents::ensure_current(&agent)?;
            let found = inventory(&project_root);
            let missing = found.missing(&spec.skills.required);
            if !missing.is_empty() && !allow_missing_skills {
                return invalid(format!(
                    "role {role} is missing required skills: {}; install them or pass --allow-missing-skills",
                    missing.join(", ")
                ));
            }
            let apply: Vec<String> = spec
                .skills
                .required
                .iter()
                .chain(&spec.skills.optional)
                .filter(|skill| found.names.contains(*skill))
                .cloned()
                .collect();
            let template = brief::load_template(&plugin.root, &plugin.profile.brief_language)?;
            let mut context = Vec::new();
            for id in &depends_on {
                let dependency = assignment::get(journal.conn(), id)?;
                if let Some((body, _)) = report::get(journal.conn(), id)? {
                    context.push(ContextItem {
                        assignment_id: dependency.id,
                        role: dependency.role,
                        outcome: body.outcome,
                        summary: body.summary,
                    });
                }
            }
            let references = plugin
                .root
                .join("skills")
                .join("fox")
                .join("references")
                .display()
                .to_string();
            let language = plugin.profile.brief_language.clone();
            let (created, text) = assignment::create_with_brief(
                &mut journal,
                &current,
                NewAssignment {
                    role,
                    kind: spec.kind,
                    profile: Some(agent.profile.clone()),
                    expected_models: vec![agent.model.clone()],
                    brief_sha256: None,
                    role_sha256: Some(agent.role_sha256.clone()),
                    allowed_paths: paths,
                    depends_on,
                    previous_id: previous,
                },
                |draft| {
                    let body = brief::render(
                        &template,
                        &BriefInput {
                            run: &current,
                            assignment: draft,
                            references_dir: &references,
                            skills: &apply,
                            missing_skills: &missing,
                            goal: &goal,
                            context: &context,
                        },
                    )?;
                    Ok(BriefDraft { language, body })
                },
            )?;
            eprintln!(
                "foxmorrow: {} profile={} model={}",
                created.id, agent.profile, agent.model
            );
            print!("{text}");
        }
        Command::Brief {
            assignment: id,
            run: run_id,
        } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (_, journal) = run::open_run(&project_root, run_id.as_deref())?;
            let Some((_, text)) = assignment::brief(journal.conn(), &id)? else {
                return Err(Error::NotFound(format!("brief for {id}")));
            };
            print!("{text}");
        }
        Command::Check {
            assignment: id,
            run: run_id,
            command,
        } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (_, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
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
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let body = report::submit(&mut journal, &current, &id, &input)?;
            print_json(
                &serde_json::json!({ "assignment": id, "status": body.status, "outcome": body.outcome }),
            )?;
        }
        Command::Status { run: run_id } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, journal) = run::open_run(&project_root, run_id.as_deref())?;
            print_json(&status::status(journal.conn(), &current)?)?;
        }
        Command::Sync {
            plugin: plugin_mode,
            check,
        } => {
            let plugin = Plugin::load(cli.plugin_root)?;
            let report = if plugin_mode {
                let definitions =
                    agents::plugin_definitions(&plugin.root, &plugin.catalog, &plugin.profile)?;
                agents::sync(&definitions, &plugin.root.join("agents"), None, check)?
            } else {
                let project_root = project::for_init(cli.project, &cwd);
                let project_profile = ProjectProfile::load(&project_root)?;
                let definitions = agents::project_definitions(
                    &plugin.root,
                    &project_root,
                    &plugin.catalog,
                    &plugin.profile,
                    &project_profile,
                )?;
                agents::sync(
                    &definitions,
                    &project_root.join(".devin").join("agents"),
                    Some(PROJECT_PREFIX),
                    check,
                )?
            };
            print_json(&report)?;
            if check && !report.stale.is_empty() {
                return Ok(ExitCode::from(1));
            }
        }
        Command::Doctor => {
            let plugin = Plugin::load(cli.plugin_root)?;
            let project_root = project::for_init(cli.project, &cwd);
            let project_profile = ProjectProfile::load(&project_root)?;
            let found = inventory(&project_root);
            let plugin_definitions =
                agents::plugin_definitions(&plugin.root, &plugin.catalog, &plugin.profile)?;
            let mut stale =
                agents::sync(&plugin_definitions, &plugin.root.join("agents"), None, true)?.stale;
            let project_definitions = agents::project_definitions(
                &plugin.root,
                &project_root,
                &plugin.catalog,
                &plugin.profile,
                &project_profile,
            )?;
            stale.extend(
                agents::sync(
                    &project_definitions,
                    &project_root.join(".devin").join("agents"),
                    Some(PROJECT_PREFIX),
                    true,
                )?
                .stale,
            );
            print_json(&serde_json::json!({
                "plugin_root": plugin.root,
                "roles": plugin.catalog.roles.len(),
                "brief_language": plugin.profile.brief_language,
                "family_rule": plugin.profile.family_rule(&project_profile),
                "skills_source": found.source,
                "missing_required_skills": missing_required(&plugin.catalog, &found),
                "stale_agents": stale,
            }))?;
            if !stale.is_empty() {
                return Ok(ExitCode::from(1));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn devin_binary() -> PathBuf {
    std::env::var_os("FOXMORROW_DEVIN_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("devin"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn inventory(project_root: &Path) -> Inventory {
    skills::inventory(project_root, &devin_binary(), &home_dir())
}

fn missing_required(catalog: &Catalog, found: &Inventory) -> BTreeMap<String, Vec<String>> {
    catalog
        .roles
        .iter()
        .map(|(name, spec)| (name.clone(), found.missing(&spec.skills.required)))
        .filter(|(_, missing)| !missing.is_empty())
        .collect()
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
