---
name: update-software-factory
description: Changes the SoftwareFactory itself (scripts/factory.sh, the agents and skills in .claude/, the rules of CLAUDE.md), then updates its documentation and brings the software in line with the new rules. Not for the software's requirements (use req-new or req-update).
argument-hint: <description of the change to the factory>
disable-model-invocation: true
---

Change to the factory requested by the user: $ARGUMENTS

1. **Check that the factory can be changed now.**
   - The main checkout is on `master` with no modified tracked file (`git status --porcelain --untracked-files=no` is empty).
   - No branch is in progress: `git branch --no-merged master --list "REQ-*" "factory-*" "conform-*"` is empty.
   - Otherwise, tell the user what blocks and stop.
2. **Write the change request** in English to `.factory/factory-change.md`. The user may describe it in any language: translate it, and keep talking to the user in their own language.
   - What to change in the factory, and why. Keep the user's intent, and do not add anything they did not ask for.
   - If the request is too vague to be applied without guessing, ask the user first.
3. **Show** the user a short summary of the request.
4. **Run the whole chain** without asking for confirmation, as a background Bash command (it can take a long time):
   `./scripts/factory.sh factory .factory/factory-change.md > .factory/reports/factory-change.log 2>&1`
   If `docker info` fails with a permission error while the user is in the `docker` group (the session started before they were added), wrap the command in `sg docker -c '…'`.
   Tell the user right away that the chain is running, and the log path.
5. **Report when the command ends.** Read the log and the reports it points to, then tell the user:
   - F1: the files the factory maintainer changed, and the branch;
   - F2: what changed in the diagram (or that it was already up to date);
   - F3: the new `master` commit;
   - F4: either `COMPLIANT`, or what the conformity branch changed in the software and the result of its test, merge and regression; and any conflict between a requirement and the new rules (the requirement wins);
   - if the chain stopped: at which stage, why, the report path, and what is already on `master` (the factory change is merged at F3, before F4 runs).
