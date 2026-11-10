# Assignment contract

`foxmorrow` below means the absolute CLI path from the brief or from `SKILL.md`.

## Checks

`foxmorrow check -a <assignment> -- <command> [args…]` runs the command once in the current directory and records a receipt: cwd, argv, exit code, time, a hash and the tail of the output. It exits with the command's exit code. Reference the printed `chk-…` ID as evidence. If a check was interrupted, inspect what it actually did before running it again.

## Report

Each assignment ends with exactly one `foxmorrow report -a <assignment>` call that reads JSON from stdin:

```json
{
  "summary": "what was done and found",
  "status": "completed",
  "outcome": "passed",
  "criteria": {"ac-1": {"status": "passed", "reason": "why", "evidence": ["chk-001"]}},
  "evidence": ["chk-001", "<tool call id>"],
  "resolves": ["a-03"]
}
```

- `summary` is a non-empty string.
- `status` is `completed` or `interrupted`; interrupted work cannot have outcome `passed`.
- `outcome` and each criterion `status` are `passed`, `failed` or `unverified`.
- `criteria` is required for QA and covers every run criterion; other roles include the criteria they address.
- `evidence` references `chk-…` receipts of the same assignment, with exit code 0 for passed claims, or IDs of your own tool calls.
- `resolves` lists earlier negative reports that this report supersedes.
- A root stop report adds `reason` and an `unverified` list of criterion IDs.

A submitted report cannot be changed.

## Next action

Root reads `foxmorrow verify -a <assignment>`: `accepted`, `rejected` with the failed claims, or `unverified` when the session evidence is missing. A completed report with outcome `failed` does not end the task; root chooses the next assignment from the findings by the routing table in `orchestrator.md`.
