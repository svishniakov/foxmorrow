# Orchestration procedure

`foxmorrow` below means the absolute CLI path from `SKILL.md`.

## Start

1. Run the `init` command chosen in `SKILL.md`. It refuses a plan or a document basis that is not approved or has open questions; report that to the user and stop.
2. Read the `init` output:
   - `criteria` are the acceptance criteria of the run; every later report covers them by ID;
   - `missing_skills` lists required skills that are not installed; ask the user whether to install them or continue without them;
   - `root_session_id` and `root_model` empty means the session evidence is not linked yet; `verify` will retry, but tell the user if it stays empty.
3. Decide the assignments before starting them: authors with disjoint allowed paths, then one review, then QA. Use `architect` or `product-manager` first only when the task needs a decision they own.

## Assignments

1. Create each assignment with `foxmorrow assign --role <role> --goal "<goal>" --paths <paths…> [--depends-on <ids…>] [--previous <id>]`. The goal names the result and points to the sections of the plan, PRD or ADR to follow; do not copy those documents into it.
2. `assign` prints the brief on stdout and `profile=… model=…` on stderr. Start `run_subagent` with that profile and the brief as the task, verbatim, in the foreground. Authors may need permissions; ask the user before starting background work.
3. When the subagent returns, run `foxmorrow verify -a <id>`.
   - `accepted`: continue.
   - `rejected`: read the failed claims, then create a new assignment with `--previous <id>` for the right owner.
   - `unverified`: the evidence is missing; check `foxmorrow doctor` and the stderr reason, then retry `verify` or tell the user what cannot be proven.
4. After all authors are accepted, assign `reviewer` with `--depends-on` on every author assignment. After an accepted review with outcome `passed`, assign `qa-verifier` with `--depends-on` on the review.
5. `foxmorrow status` shows the run at any time. After context compaction, read it before acting.

## Routing findings

| Finding | Next assignment |
|---|---|
| Implementation defect | the author role, `--previous` on the author assignment |
| Architecture problem | `architect` |
| Disputed architectural choice | `supervising-architect` |
| Insufficient or doubtful verification | the verifier again, or `senior-qa-verifier` for a QA adequacy dispute |
| Missing information or a manual action | ask the user |

After a correction, repeat the affected review and QA with new assignments.

## Close

1. Close the run with `foxmorrow finalize --report <qa assignment>` after an accepted QA report.
2. If the run cannot reach a passing QA, create a root assignment with `foxmorrow assign --root --paths <paths…>`, submit a root report with `outcome` `failed` or `unverified`, a `reason` and the `unverified` criterion IDs, verify it, and finalize with that report.
3. Tell the user the final outcome, the root model from the final record and what remains unverified.
4. If a plan or ADR turns out wrong during implementation, stop the dependent work, finalize the run as not passed and propose a separate Doc Flow run to revise the document.
