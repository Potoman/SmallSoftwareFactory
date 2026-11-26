# SoftwareFactory

This repository is built by a chain of agents, from the requirements written in `requirements/`.

## Process

```
requirements/REQ-xxx-*.md (new, not committed)
        │
        ▼
 1. req-checker        → contradicts an existing requirement?  CONFLICT = stop
        │ OK
        ▼
 2. implementer        → branch <ID>-<short-title> (requirement + code + tests committed)
        │
        ▼
 3. candidate commit   = master + merge --no-ff of the branch (temporary worktree)
    │ conflict → back to the implementer (it merges master into its branch)
    ▼
    branch-tester      → tests THAT commit against "How to test" + the whole suite
        │ PASS                          │ FAIL → back to the implementer (with the report)
        ▼
 4. git merge --ff-only <candidate>: master becomes exactly the tested commit
    (refused if master moved in the meantime → new candidate, new test)
        │
        ▼
 5. regression-tester  → retests ALL requirements present on master
```

Orchestration is done by `scripts/factory.sh` (run `./scripts/factory.sh --help`). The agents live in `.claude/agents/`.

## Environment

- Everything runs in a Unix environment (Linux, or WSL on a Windows machine). Use POSIX paths and bash.
- No Windows-specific tooling: no PowerShell, `.bat`/`.cmd` files or Windows paths. Scripts are bash, with a `#!/usr/bin/env bash` shebang.
- Text files use LF line endings (enforced by `.gitattributes`).

## Conventions

- **Requirement**: `requirements/<ID>-<short-title>.md`, following `requirements/_TEMPLATE.md`.
  - ID: `REQ-` followed by 3 digits (`REQ-001`). Unique, never reused.
  - A requirement is "merged" when its file is present on `master`. Until then it lives in its branch.
- **Branch**: `<ID>-<short-title-in-kebab-case>`, e.g. `REQ-001-email-login`. Only `[a-z0-9-]` after the ID.
- **Tests**: every requirement is covered by automated tests whenever possible. Their name contains the lowercase ID (e.g. `tests/req_001_*.py`, `req-001.test.ts`).
- **Main branch**: `master`.
- **Reports**: agent reports go to `.factory/reports/`. This folder is ignored by git.
- **Language**: everything in the repository (code, comments, requirements, commit messages, reports) is written in English.

## Tech stack

> To be defined. While this section is empty, the first implemented requirement picks the stack. It must then describe it here, together with the command that runs the whole test suite.

- Language: _to be defined_
- Test command (whole suite): _to be defined_

## Rules for all agents

- Never modify an existing requirement, except the one being implemented.
- Never modify `master`: only the script moves it. No push, no rebase, no `reset --hard`. Single exception: the implementer, in conflict mode, may run `git merge master` **inside its own branch**.
- End the final answer with a `VERDICT: <VALUE>` line (values defined by each agent). The script reads this line.
