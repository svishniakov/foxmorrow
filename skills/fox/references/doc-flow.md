# Doc Flow

Documents are prepared with the same run mechanics as code; the product of the run is the document. Each document is created or substantively revised in its own run, started with `foxmorrow init --doc <type> --based-on <documents…> --paths <document>`.

## Document chain

```
BRD → ADR → PRD (one per feature) ─┬→ Implementation Plan → implementation
                                   └→ ADR (architecture changes) → Implementation Plan → implementation
```

Every document starts with a header:

```
Document status: draft
Based on: [title](relative/path.md), …
```

A Russian status line `Статус: черновик …` or `Статус: утверждён …` is read the same way; only its first word counts. Links in `Based on` may point into other repositories of the same workspace.

The first ADR is based on the BRD, a PRD on the BRD and the ADR, a feature ADR on its PRD, a plan on its PRD and, if present, on the feature ADR. A BRD may have no basis.

## Document approval

A document cannot have status `approved` while it contains an open question, whatever its importance. A question is closed when a justified answer or a decision of an authorized participant is recorded. Deleting the question, hiding it in another section or replacing it with an unconfirmed assumption does not close it.

Drafts may contain open questions; list them under a heading `Open questions`. An action that is planned but not done yet is not an open question when the decision, owner and conditions are defined. An unresolved choice, an unknown contract or an unconfirmed premise stays an open question.

Only the current revision with status `approved` and no open questions can be the basis for further documents or implementation. `foxmorrow init` checks the whole `Based on` chain. A review or QA verdict is `PASS` or `FAIL`; neither approves the document. Root sets `approved` only after the user approves the verified document.

## Routing

Root coordinates the document run: it chooses roles, creates assignments, passes the current text and the previous verdict, routes findings to the owner and decides the next step. Authors write content; reviewer and QA accept it independently. The order for every type:

1. The author writes the document and, for Russian text, applies `humanizer-ru`.
2. A separate `ai-slops-hunter` assignment edits the whole document and preserves decisions, numbers, conditions and technical syntax.
3. An independent `reviewer` checks the document against its sources and its acceptance criteria, including that the editing preserved the meaning.
4. After `PASS`, an independent `qa-verifier` checks coverage and readiness.
5. Root finalizes the run and gives the user a link to the verified document (Markdown link with the absolute path), its status and any open questions.

A material change after review repeats the editing and the affected checks. A missing required role or skill leaves its step not done; the author's self-check does not replace independent acceptance.

## BRD

`product-manager` writes the BRD: the business problem, target users or segments, value, success metrics, scope and out-of-scope, constraints such as budget, timeline and compliance, risks and labelled assumptions. When market data is needed, root first assigns `marketing-growth-strategist` and gives the author its accepted report through `--depends-on`.

### BRD acceptance criteria

- The business problem, the target users and the value are stated and consistent.
- Success metrics are measurable and tied to the problem.
- Scope and out-of-scope are explicit; constraints are listed with their source.
- Market claims rest on the assigned market input or are labelled as assumptions.
- Risks and assumptions are labelled; status and open questions reflect the facts.

## PRD

`product-manager` writes the PRD for one coherent block of functionality, even when the requirements are already settled. Independent capabilities become sequential PRDs with their own results and order. The PRD explains the goal and context, the user scenario with the expected behavior, constraints, boundaries and acceptance conditions. Each requirement has one main place. It keeps significant scenarios, permissions, errors, business constraints, data safety and measurable outcomes, and leaves architecture, protocols and verification methods to the ADR and the plan.

After a PRD is approved, root assigns `architect` in the PRD run to decide whether a feature ADR is needed. The decision and its reason are part of that accepted report. When the architecture changes and there is no new ADR, the plan cannot be approved.

### PRD acceptance criteria

- The scope is one coherent block of functionality with a clear result.
- The goal, context, user scenario with expected behavior, constraints, boundaries and acceptance conditions are present; each requirement has one main place.
- Significant scenarios, permissions, errors, business constraints, data safety and measurable outcomes are preserved; implementation design is left to the ADR and the plan.
- `Based on` links the BRD and the ADR; status and open questions reflect the facts.

## ADR

`architect` writes the ADR for a significant decision or closely related decisions that must be assessed together; independent decisions get separate ADRs. The ADR states the problem and source requirements, constraints, the chosen decision, significant alternatives and consequences, the responsibility of components, data owners and contracts. Each decision has one main place; decisions for one service stay in its section. The ADR is not a list of files or tasks.

The architect applies `arch-simplicity-guard` to the draft. A disputed choice goes to `supervising-architect` at once, through root; it decides within the assignment and updates the ADR itself. The reviewer applies the guard again in its own session.

### ADR acceptance criteria

- The decision is tied to the source requirements and confirmed constraints; assumptions and unknowns are labelled.
- The choice, its scope, the responsibility of affected components and significant contracts are defined unambiguously.
- Applicable alternatives are considered; the reasons and the significant positive and negative consequences are explained.
- Requirements, risks, guarantees and verifiable invariants are preserved; there are no unnecessary mechanisms or contradictions with current decisions.
- The structure is logical and the ADR is not replaced by a plan of file changes; status and open questions reflect the facts.

## Plan

`architect` writes the Implementation Plan from the approved PRD and ADRs and the current code, even when the approach is already chosen. A contradiction with an ADR goes back to the ADR process through root; a new decision is not hidden in a task. The plan separates local development from external setup and live integration checks, each with its own acceptance criteria; local work and its QA do not depend on external accounts, credentials or remote CI. A new stage is justified only by a real dependency, a contract change, a data migration or a separately verifiable result.

The plan has a section `Критерии приёмки` or `Acceptance criteria` with a list; `foxmorrow init --plan` takes the run criteria from it. External setup and live checks are separate sections or separate plans so that they get their own runs.

The architect applies `arch-simplicity-guard`; disputed choices go to `supervising-architect` as for the ADR. Before the final review, a separate `qa-verifier` assignment reviews the requirements coverage of the draft.

### Plan acceptance criteria

- The scope and stages implement the relevant requirements and accepted decisions; nothing is lost and no ADR is revised silently.
- Each stage has a result, a change area, responsible roles, real dependencies, criteria and sufficient checks.
- Local development has its own acceptance without external credentials or service availability; external prerequisites, owners and live checks are defined separately.
- The plan matches the current code and known contracts, keeps significant risks and adds no unjustified mechanisms or procedures.
- `Based on` links the PRD and, if present, the feature ADR; status and open questions reflect the facts.
