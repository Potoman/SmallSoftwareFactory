---
name: req-new
description: Writes a new SoftwareFactory requirement in requirements/ with the next free ID, then runs the whole factory chain on it (consistency check, implementation, tests, merge, regression). Use when the user wants to add, write or describe a new requirement or feature.
argument-hint: <free-form description of the feature>
---

Need expressed by the user: $ARGUMENTS

1. **Find the next free ID.** Take the highest `REQ-NNN` number used among:
   - the files on master: `git ls-tree --name-only master requirements/`;
   - the files in the `requirements/` folder;
   - the branches: `git branch --list "REQ-*"`.

   The next ID is that number + 1, on 3 digits. If there is none, it is `REQ-001`.
2. **Write the requirement** in English, following `requirements/_TEMPLATE.md`. The user may describe the need in any language: translate it, and never copy non-English text into the file. Keep talking to the user in their own language.
   - a short title;
   - a precise description that covers edge cases;
   - a "How to test" section made of verifiable steps, each with a measurable expected result.

   Do not ask the user whether it is a feature or a technical constraint, and do not write a `kind` field: the factory decides that during the consistency check. Write it the same way in both cases.

   If the need is too vague to write verifiable tests, first ask the user the necessary questions.
3. **Save** the file as `requirements/<ID>-<short-kebab-title>.md`. Do not commit it: the implementer will commit it on its branch.
4. **Start the whole chain** without asking for confirmation. Do not call the `req-checker` yourself: the `run` command does the consistency check as its first stage and stops there on a conflict.
   - First make sure the main checkout is on `master` with no modified tracked files (`git status --porcelain --untracked-files=no` must be empty); otherwise the script refuses to start. If it is not clean, tell the user what is modified and stop.
   - Run, as a background Bash command (it can take a long time):
     `./scripts/factory.sh run requirements/<file>.md > .factory/reports/factory-<ID>.log 2>&1`
   - Tell the user right away: the requirement file path, that the chain is running, and the log path.
5. **Report when the command ends** (you are notified when the background command exits). Read the log and the reports it points to, then tell the user:
   - if it ended with `Done.`: the kind the factory chose (feature or constraint), the branch merged into `master` and the regression result;
   - if it stopped at the consistency check (`CONFLICT`): the contradictions found, and a suggested rewording of the requirement (or a split into a constraint and a feature, if it mixed both);
   - if it stopped later: at which stage (implementation, candidate test, merge, regression), why, and the report path.
