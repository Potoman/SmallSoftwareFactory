---
name: req-update
description: Changes an already merged SoftwareFactory requirement by editing its file in requirements/, then runs the whole factory chain on it (consistency check, implementation, tests, merge, regression). Use when the user wants to modify, amend, relax, extend or fix an existing requirement.
argument-hint: <REQ-xxx> <free-form description of the change>
---

Change requested by the user: $ARGUMENTS

1. **Find the requirement.** The first argument is its ID (`REQ-` + 3 digits); the rest is the change. If there is no ID, or no change, ask the user.
   - It must be merged: `git ls-tree --name-only master requirements/ | grep "^requirements/<ID>-"`.
   - If it is not on `master` yet (only a file in `requirements/` or a branch in progress), stop: tell the user to edit that file directly, or to wait until it is merged.
   - If an update of the same ID is already in progress (a branch `<ID>-*` not merged into master: `git branch --no-merged master --list "<ID>-*"`), stop and tell the user.
   - The main checkout must be on `master`, and no tracked file other than this requirement may be modified (`git status --porcelain --untracked-files=no`). Otherwise, tell the user what is modified and stop.
   - If the requirement file already has uncommitted changes, show them (`git diff master -- <file>`) and ask whether to build on them or start again from the version on `master`.
2. **Edit the requirement file in place**, `requirements/<ID>-<title>.md`, in English. The user may describe the change in any language: translate it, and never copy non-English text into the file. Keep talking to the user in their own language.
   - Keep the `id` and the file name. Change the title only if the user asks.
   - Remove the `kind` line from the frontmatter: the factory decides the kind again.
   - Change only what the request is about, and keep the rest word for word. Update every "How to test" step the change affects, so that each step stays verifiable with a measurable expected result.
   - If the change is too vague to write verifiable tests, first ask the user the necessary questions.
   - Do not commit: the implementer takes the change to its branch.
3. **Show the change** to the user as a short summary of what is added, removed or changed (`git diff master -- <file>`).
4. **Start the whole chain** without asking for confirmation. Run, as a background Bash command (it can take a long time):
   `./scripts/factory.sh update requirements/<file>.md > .factory/reports/factory-<ID>-update.log 2>&1`
   Tell the user right away: the requirement file, that the chain is running, and the log path.
5. **Report when the command ends** (you are notified when the background command exits). Read the log and the reports it points to, then tell the user:
   - if it ended with `Done.`: the kind the factory chose, the branch merged into `master`, what the implementer changed besides the requirement, and the regression result;
   - if it stopped at the consistency check (`CONFLICT`): the contradictions found and a suggested rewording. The change stays uncommitted in the requirement file, so it can be reworked; until then, it blocks any other factory run. To drop it: `git restore <file>`;
   - if it stopped later: at which stage, why, and the report path. Say where the change is now: still uncommitted on `master` (implementer blocked before committing), on the update branch, or already on `master` (the regression failed after the merge).
