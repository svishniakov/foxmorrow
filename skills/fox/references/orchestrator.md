# Orchestration procedure

`foxmorrow` below means the absolute CLI path from `SKILL.md`.

## Start

1. Run the `init` command chosen in `SKILL.md` from the project root. The project root is the folder the work happens in; it does not have to be a git repository. When it contains several repositories of one product, `init` records the git state of every repository one level below it, or of the ones you pass with `--repo`; a folder without repositories gets a run without git state. Write allowed paths relative to the project root (`app-saas/docs/…`) and run each `foxmorrow check` in the folder it tests.
   `init` refuses a plan or a document basis that is not approved or has open questions. Show the user each listed document with its status and its open questions, and ask the user to approve it. A document becomes approved only when the user closes its open questions and approves it; then record the approval in the document's status line (`Document status: approved` or `Статус: утверждён`) and run `init` again. Do not approve documents yourself and do not start the work on a draft basis.
2. Read the `init` output:
   - `criteria` are the acceptance criteria of the run; every later report covers them by ID;
   - `missing_skills` lists required skills that are not installed; ask the user whether to install them or continue without them;
   - `root_session_id` and `root_model` empty means the session evidence is not linked yet; `verify` will retry, but tell the user if it stays empty.
3. Read the project memory files that `init` lists with `exists: true` (see [Project memory](#project-memory)). Apply relevant lessons and notes when you plan assignments. Mention open todo items that touch this task to the user.
4. Decide the assignments before starting them: authors with disjoint allowed paths, then one review, then QA. Use `architect` or `product-manager` first only when the task needs a decision they own.

## Assignments

1. Create each assignment with `foxmorrow assign --role <role> --goal "<goal>" --paths <paths…> [--depends-on <ids…>] [--previous <id>]`. The goal names the result and points to the sections of the plan, PRD or ADR to follow; do not copy those documents into it.
   Before each assignment, go through `lessons.md` and `notes.md` again for this role, its paths and its goal. Pass every entry that affects how the role works, checks or reports with `--memory "<entry>"`, verbatim with its source tag; the brief lists them under «Project memory». Subagents do not read memory, so an entry you do not pass does not reach them. Leave out entries that do not concern this assignment, and do not paraphrase an entry into the goal instead.
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
3. Update the project memory as described below, then tell the user the final outcome, the root model from the final record, what remains unverified and what you added to memory.
4. If a plan or ADR turns out wrong during implementation, stop the dependent work, finalize the run as not passed and propose a separate Doc Flow run to revise the document.

## Project memory

Memory keeps what one run learned for the next runs in the same project. Only root reads and writes it; it lives in `.foxmorrow/memory/` and is meant to be committed with the project. Every entry is one list item that starts with its source, for example `- [r-1a2b3c a-04] …`. Keep entries short and factual, never store secrets, and correct or delete an entry when it becomes wrong.

- `todo.md`: work that was deferred or left unverified: the run's `unverified` criteria, findings outside the allowed paths, follow-up tasks the user postponed. Remove an item when a later run finishes it.
- `lessons.md`: a rule for future runs, added only when it is confirmed by a rejected verification, a failed review or QA, or a user correction in this run. Write it as an instruction (`Cite only receipts of your own assignment.`), not as a story, and do not repeat an existing lesson.
- `notes.md`: stable project facts that a run established and other runs need: build and test commands, required services, non-obvious constraints. Record where the fact came from.

Update memory after `finalize`, outside the run's allowed paths, and only with facts from this run. Do not write memory when the user asked not to change files. Memory is not evidence: a claim from memory still needs checks in the current run.
