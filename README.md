# foxmorrow

foxmorrow runs a task in Devin through role subagents and accepts a result only when the evidence agrees with it. Each role has its own model. Root, the session you talk to, creates assignments, starts subagents with a brief, and checks every report against the foxmorrow journal and the Devin session database before moving on.

## What it proves

For every report, `foxmorrow verify` checks:

| Claim | Meaning |
|---|---|
| C1 | The subagent received the recorded brief and role text |
| C2 | The report was submitted from that subagent's own chain |
| C3 | Every answer in the chain came from the model of the role's profile |
| C4 | Referenced checks and tool calls exist and passed |
| C5 | A reviewer or QA did not edit files |
| C6 | Review and QA depend on the work they check, on a different model family |

An unknown Devin database layout gives `unverified`, never a guessed `passed`. Accepted reports are immutable, and `finalize` checks the stored snapshots again.

It does not prove what it cannot see: a reviewer can still change files through shell commands, and a user who edits `sessions.db` or the journal by hand can forge results.

## Requirements

* Devin CLI or Devin Desktop on macOS, Linux or Windows x64. Devin Cloud sessions are not supported in v1: plugin subagents do not load there.
* `curl` or `wget` on macOS and Linux, PowerShell 5.1 on Windows.
* The `arch-simplicity-guard` skill, required by `architect`, `supervising-architect` and `reviewer`. Install it into `~/.agents/skills/arch-simplicity-guard` from https://github.com/svishniakov/arch-simplicity-guard.

## Install

```sh
devin plugins install svishniakov/foxmorrow
```

The first foxmorrow command downloads the binary of the plugin's version from GitHub Releases, checks its SHA-256 against `checksums.json` and caches it in `~/.cache/foxmorrow/` (`%LOCALAPPDATA%\foxmorrow\` on Windows). Update with `devin plugins update foxmorrow`.

## Use

Start a message with `$fox` or `$foxmorrow`, or run `/foxmorrow:fox`:

```text
$fox Implement the plan docs/implementation/impl-004-health.md
$fox Write the PRD for team invitations based on docs/brd.md and docs/adr/adr-001.md
$fox Fix the login timeout; acceptance criteria are in docs/tasks/login.md#Acceptance criteria
```

| Flow | Run starts with |
|---|---|
| Implement an approved plan | `foxmorrow init --plan <plan>` takes the criteria from the plan and refuses a plan whose `Based on` chain is not approved |
| Write or revise a BRD, ADR, PRD or plan | `foxmorrow init --doc <type> --based-on <documents>` uses the Doc Flow criteria of that type |
| Any task with written acceptance criteria | `foxmorrow init --criteria <file>#<heading>` |

Documents carry `Document status: draft|approved` and `Based on:` in their header. A document with open questions cannot be approved, and only you approve documents.

## Project memory

Root keeps memory between runs in `.foxmorrow/memory/`: `todo.md` for deferred and unverified work, `lessons.md` for rules confirmed by a rejected check or your correction, `notes.md` for project facts such as build commands. It reads them when a run starts, passes the entries that concern an assignment into its brief with `foxmorrow assign --memory`, and updates the files after `finalize`. Commit `memory/` with the project; `foxmorrow init` adds a `.gitignore` that keeps run journals and the session file out of git. Memory is not evidence: every run proves its own results.

## Models

Roles have tiers; `profiles/devin.json` maps tiers to Devin models. Override them for one project in `.foxmorrow/profile.json` and run `foxmorrow sync` to generate project roles:

```json
{ "tiers": { "worker": "swe-2-max" }, "roles": { "reviewer": "gpt-6-astra-high" }, "family_rule": true }
```

With `family_rule` on, reviewer and QA must use a model family other than the author's.

## Diagnose

`foxmorrow doctor` shows the plugin root, where the skill list came from, missing required skills and stale role files. `foxmorrow status` shows the current run. Hooks are best effort: when they do not run, `verify` still reads `sessions.db`.

## Release

1. Set the same version in `Cargo.toml`, `.devin-plugin/plugin.json` and `checksums.json` (with an empty `binaries` list), commit to `main`.
2. Push the tag `v<version>`. The release workflow builds five binaries, publishes the GitHub Release and commits the filled `checksums.json` to `main`.

## License

MIT
