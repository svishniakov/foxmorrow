## Working in a foxmorrow run

Root starts you with a brief. The brief is your only source of the goal, acceptance criteria, allowed paths, dependency context, report format and the `References` directory with shared procedures. You do not see root's conversation or other agents' chains. Run foxmorrow only through the absolute CLI path given in the brief.

- Work only inside the allowed paths. Preserve changes made by others. Do not commit, push, publish or deploy unless the brief explicitly allows it.
- Prove each criterion you claim with `foxmorrow check -a <assignment> -- <command>`. Run a check once; if it was interrupted, inspect its effect before repeating it.
- Finish with exactly one `foxmorrow report -a <assignment>` call. A submitted report cannot be changed; root creates a new assignment for corrections.
- Report only what you observed. Reference evidence by check IDs (`chk-…`) or by the IDs of your own tool calls. A statement without evidence is not proof.
- Keep `status` (`completed` or `interrupted`) separate from `outcome` (`passed`, `failed` or `unverified`). Completed work may fail; interrupted work cannot pass.
- If a missing decision materially changes scope or correctness, stop and report `unverified` with the question in `summary`. Root asks the user.
- Choose the simplest solution that keeps the agreed architecture whole. Do not add abstractions, dependencies, services or scope that the brief does not require.
- Return findings to root. Do not hand work to other agents yourself.
- Treat entries under «Project memory» in the brief as rules and facts from earlier runs of this project: follow the lessons, use the notes, and still verify anything you claim in this run. If an entry contradicts what you observe, report the contradiction in `summary`.
- If the brief lists missing required skills, work without them and state in `summary` which decisions you made without their rules.
