mod project;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use foxmorrow_core::assignment::{self, Assignment, BriefDraft, NewAssignment};
use foxmorrow_core::brief::{self, BriefInput, ContextItem};
use foxmorrow_core::catalog::Catalog;
use foxmorrow_core::claims::{self, ChainObservation, ModelFamilies};
use foxmorrow_core::document;
use foxmorrow_core::error::invalid;
use foxmorrow_core::finalize;
use foxmorrow_core::journal::Journal;
use foxmorrow_core::model::Kind;
use foxmorrow_core::model::VerificationState;
use foxmorrow_core::run::{self, NewRun};
use foxmorrow_core::{Error, Result, baseline, criteria, receipt, report, status};
use foxmorrow_devin::agents::{self, PROJECT_PREFIX};
use foxmorrow_devin::hook;
use foxmorrow_devin::profile::{Profile, ProjectProfile};
use foxmorrow_devin::reader::{self, ChainQuery, Reader};
use foxmorrow_devin::schema_map::SchemaMap;
use foxmorrow_devin::skills::{self, Inventory};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "foxmorrow", version, about = "Provable multi-agent runs")]
struct Cli {
    #[arg(long, global = true)]
    project: Option<PathBuf>,
    #[arg(long, global = true)]
    plugin_root: Option<PathBuf>,
    #[arg(long, global = true)]
    sessions_db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, conflicts_with_all = ["plan", "doc"])]
        criteria: Option<String>,
        #[arg(long, conflicts_with = "doc")]
        plan: Option<PathBuf>,
        #[arg(long, value_enum)]
        doc: Option<DocType>,
        #[arg(long = "based-on", num_args = 1..)]
        based_on: Vec<PathBuf>,
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
        #[arg(long = "memory", num_args = 1..)]
        memory: Vec<String>,
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
    Verify {
        #[arg(short = 'a', long = "assignment")]
        assignment: String,
        #[arg(long)]
        run: Option<String>,
    },
    Finalize {
        #[arg(long = "report")]
        report: String,
        #[arg(long)]
        run: Option<String>,
    },
    Hook {
        event: String,
    },
    Sync {
        #[arg(long)]
        plugin: bool,
        #[arg(long)]
        check: bool,
    },
    Doctor,
}

#[derive(Clone, Copy, ValueEnum)]
enum DocType {
    Brd,
    Adr,
    Prd,
    Plan,
}

impl DocType {
    fn heading(self) -> &'static str {
        match self {
            DocType::Brd => "BRD acceptance criteria",
            DocType::Adr => "ADR acceptance criteria",
            DocType::Prd => "PRD acceptance criteria",
            DocType::Plan => "Plan acceptance criteria",
        }
    }
}

struct CriteriaSource {
    path: String,
    heading: String,
    markdown: String,
}

const MEMORY_FILES: [&str; 3] = ["todo.md", "lessons.md", "notes.md"];

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
            plan,
            doc,
            based_on,
            paths,
            repos,
            session_id,
        } => {
            let root = project::for_init(cli.project, &cwd);
            let plugin = Plugin::load(cli.plugin_root)?;
            let source = criteria_source(&root, &plugin.root, reference, plan, doc, &based_on)?;
            let project_profile = ProjectProfile::load(&root)?;
            let found = inventory(&root);
            let skills_summary = missing_required(&plugin.catalog, &found);
            let repos = if repos.is_empty() {
                vec![root.clone()]
            } else {
                repos
            };
            let mut baselines = Vec::new();
            for repo in &repos {
                baselines.push(baseline::collect(repo)?);
            }
            let evidence = open_reader(cli.sessions_db, &plugin.root);
            let directory = root
                .canonicalize()
                .unwrap_or_else(|_| root.clone())
                .display()
                .to_string();
            let session = session_id
                .or_else(|| hook::read_session(&root).map(|record| record.session_id))
                .or_else(|| {
                    evidence
                        .as_ref()
                        .ok()
                        .and_then(|reader| reader.latest_session_in(&directory).ok().flatten())
                });
            let root_model = match (&evidence, &session) {
                (Ok(reader), Some(session)) => reader.root_model(session).ok().flatten(),
                _ => None,
            };
            let state_dir = run::foxmorrow_dir(&root);
            let (created, mut journal) = run::create_run(NewRun {
                project_root: root,
                criteria_path: source.path,
                criteria_heading: source.heading,
                criteria_markdown: source.markdown,
                allowed_paths: paths,
                baselines,
                root_session_id: session,
                family_rule: plugin.profile.family_rule(&project_profile),
                skills_summary: skills_summary.clone(),
            })?;
            let created = if root_model.is_some() {
                run::record_root(&mut journal, None, root_model.as_deref())?
            } else {
                created
            };
            let ignore = state_dir.join(".gitignore");
            if !ignore.exists() {
                std::fs::write(&ignore, "runs/\nsession\n")?;
            }
            let memory: Vec<serde_json::Value> = MEMORY_FILES
                .iter()
                .map(|name| {
                    serde_json::json!({
                        "file": format!(".foxmorrow/memory/{name}"),
                        "exists": state_dir.join("memory").join(name).is_file(),
                    })
                })
                .collect();
            print_json(&serde_json::json!({
                "run": created.id,
                "criteria": created.criteria,
                "family_rule": created.family_rule,
                "skills_source": found.source,
                "missing_skills": skills_summary,
                "root_session_id": created.root_session_id,
                "root_model": created.root_model,
                "evidence": evidence.as_ref().map_or_else(|reason| format!("unavailable: {reason}"), Reader::source),
                "memory": memory,
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
            memory,
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
            if memory.iter().any(|entry| entry.trim().is_empty()) {
                return invalid("--memory entries must not be empty");
            }
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
            let cli_path = foxmorrow_cli(&plugin.root).display().to_string();
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
                            foxmorrow: &cli_path,
                            skills: &apply,
                            missing_skills: &missing,
                            memory: &memory,
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
        Command::Verify {
            assignment: id,
            run: run_id,
        } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let plugin = Plugin::load(cli.plugin_root)?;
            let evidence = open_reader(cli.sessions_db, &plugin.root);
            let target = assignment::get(journal.conn(), &id)?;
            let session = current
                .root_session_id
                .clone()
                .or_else(|| hook::read_session(&project_root).map(|record| record.session_id));
            let observation = match (&evidence, &session) {
                (Ok(reader), Some(session)) => {
                    if current.root_model.is_none() {
                        let model = reader.root_model(session).ok().flatten();
                        run::record_root(&mut journal, Some(session), model.as_deref())?;
                    }
                    observe(reader, session, &target, &journal)
                }
                (Err(reason), _) => {
                    eprintln!("foxmorrow: no native evidence: {reason}");
                    None
                }
                (_, None) => {
                    eprintln!(
                        "foxmorrow: no native evidence: the root session id is unknown; pass --session-id to init"
                    );
                    None
                }
            };
            let extra = observation
                .as_ref()
                .map(|observed| observed.models.clone())
                .unwrap_or_default();
            let families = model_families(&journal, &plugin.profile, &extra)?;
            let verification = claims::verify(&mut journal, &current, &id, observation, &families)?;
            print_json(&serde_json::json!({
                "assignment": verification.assignment_id,
                "state": verification.state,
                "claims": verification.claims,
            }))?;
            if verification.state != VerificationState::Accepted {
                return Ok(ExitCode::from(1));
            }
        }
        Command::Finalize {
            report: id,
            run: run_id,
        } => {
            let project_root = project::for_existing(cli.project, &cwd)?;
            let (current, mut journal) = run::open_run(&project_root, run_id.as_deref())?;
            let plugin = Plugin::load(cli.plugin_root)?;
            let families = model_families(&journal, &plugin.profile, &[])?;
            print_json(&finalize::finalize(&mut journal, &current, &id, &families)?)?;
        }
        Command::Hook { event } => {
            let mut input = String::new();
            let _ = std::io::stdin().read_to_string(&mut input);
            let project_root = std::env::var_os("DEVIN_PROJECT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| project::for_init(None, &cwd));
            match hook::handle(&event, &input, &project_root, |session| {
                compact_status(&project_root, session)
            }) {
                Ok(Some(output)) => println!("{output}"),
                Ok(None) => {}
                Err(error) => eprintln!("foxmorrow: hook {event}: {error}"),
            }
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

fn open_reader(
    explicit: Option<PathBuf>,
    plugin_root: &Path,
) -> std::result::Result<Reader, String> {
    let map = SchemaMap::load(plugin_root).map_err(|error| error.to_string())?;
    let path = reader::locate(
        explicit,
        std::env::var_os("FOXMORROW_SESSIONS_DB").map(PathBuf::from),
        std::env::var_os("DEVIN_SESSIONS_DIR").map(PathBuf::from),
        &home_dir(),
    );
    Reader::open(&path, map).map_err(|error| error.to_string())
}

fn observe(
    reader: &Reader,
    session: &str,
    target: &Assignment,
    journal: &Journal,
) -> Option<ChainObservation> {
    let found = if target.kind == Kind::Root {
        reader.root_observation(session)
    } else {
        let header = assignment::brief(journal.conn(), &target.id)
            .ok()
            .flatten()
            .and_then(|(_, text)| text.lines().next().map(str::to_string));
        match (header, &target.profile) {
            (Some(header), Some(profile)) => reader.find_subagent(
                session,
                &ChainQuery {
                    profile: profile.clone(),
                    needles: vec![header],
                },
            ),
            _ => Err(Error::NotFound("the recorded brief or profile".into())),
        }
    };
    match found {
        Ok(observed) => Some(observed),
        Err(error) => {
            eprintln!("foxmorrow: no native evidence for {}: {error}", target.id);
            None
        }
    }
}

fn model_families(journal: &Journal, profile: &Profile, extra: &[String]) -> Result<ModelFamilies> {
    let mut models = claims::observed_models(journal.conn())?;
    for entry in assignment::list(journal.conn())? {
        models.extend(entry.expected_models);
    }
    models.extend(extra.iter().cloned());
    Ok(profile.families_for(models.iter()))
}

fn compact_status(project_root: &Path, session: Option<&str>) -> Option<String> {
    let (current, journal) = run::open_run(project_root, None).ok()?;
    let known = current
        .root_session_id
        .clone()
        .or_else(|| hook::read_session(project_root).map(|record| record.session_id));
    if session.is_none() || known.as_deref() != session {
        return None;
    }
    serde_json::to_string(&status::status(journal.conn(), &current).ok()?).ok()
}

fn foxmorrow_cli(plugin_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("FOXMORROW_BIN") {
        return PathBuf::from(path);
    }
    let launcher = plugin_root.join("bin").join(if cfg!(windows) {
        "foxmorrow.cmd"
    } else {
        "foxmorrow"
    });
    if launcher.is_file() {
        return launcher;
    }
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("foxmorrow"))
}

fn criteria_source(
    root: &Path,
    plugin_root: &Path,
    reference: Option<String>,
    plan: Option<PathBuf>,
    doc: Option<DocType>,
    based_on: &[PathBuf],
) -> Result<CriteriaSource> {
    if !based_on.is_empty() && doc.is_none() {
        return invalid("--based-on is used only with --doc");
    }
    if let Some(reference) = reference {
        let (file, heading) = criteria::split_reference(&reference)?;
        return Ok(CriteriaSource {
            path: file.to_string(),
            heading: heading.to_string(),
            markdown: std::fs::read_to_string(root.join(file))?,
        });
    }
    if let Some(plan) = plan {
        let path = root.join(&plan);
        document::check_chain(std::slice::from_ref(&path))?;
        let markdown = std::fs::read_to_string(&path)?;
        let Some(heading) = ["Критерии приёмки", "Acceptance criteria"]
            .into_iter()
            .find(|heading| criteria::parse_section(&markdown, heading).is_ok())
        else {
            return invalid(format!(
                "{} has no «Критерии приёмки» or «Acceptance criteria» section with list items",
                plan.display()
            ));
        };
        return Ok(CriteriaSource {
            path: plan.display().to_string(),
            heading: heading.to_string(),
            markdown,
        });
    }
    let Some(doc) = doc else {
        return invalid("one of --criteria, --plan or --doc is required");
    };
    if based_on.is_empty() && !matches!(doc, DocType::Brd) {
        return invalid("this document type needs --based-on with its approved source documents");
    }
    let basis: Vec<PathBuf> = based_on.iter().map(|path| root.join(path)).collect();
    document::check_chain(&basis)?;
    let file = plugin_root
        .join("skills")
        .join("fox")
        .join("references")
        .join("doc-flow.md");
    Ok(CriteriaSource {
        markdown: std::fs::read_to_string(&file)
            .map_err(|_| Error::NotFound(format!("Doc Flow reference {}", file.display())))?,
        path: file.display().to_string(),
        heading: doc.heading().to_string(),
    })
}
