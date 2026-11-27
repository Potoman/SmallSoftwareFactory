# SoftwareFactory

This repository is built by a chain of agents, from the requirements written in `requirements/`.

## Process

```
requirements/REQ-xxx-*.md (new, not committed)
        │
        ▼
 1. req-checker        → contradicts an existing requirement?  CONFLICT = stop
        │ OK      + classifies it: feature or constraint
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

A merged requirement is changed through the same chain: `/req-update` edits `requirements/<ID>-*.md` in place (uncommitted), then `factory.sh update <file>` checks the new version against the other requirements, has the implementer take it to a branch `<ID>-update-<summary>` and adapt the code, tests, merges and runs the regression.

Orchestration is done by `scripts/factory.sh` (run `./scripts/factory.sh --help`). The agents live in `.claude/agents/`.

## Environment

- Everything runs in a Unix environment (Linux, or WSL on a Windows machine). Use POSIX paths and bash.
- No Windows-specific tooling: no PowerShell, `.bat`/`.cmd` files or Windows paths. Scripts are bash, with a `#!/usr/bin/env bash` shebang.
- Text files use LF line endings (enforced by `.gitattributes`).

## Containers

Agents never install anything on the host: every tool the project needs (compiler, test runner…) runs in a container. Containers are **one-shot**, and images have a known lifecycle. This applies to `docker` and `podman` alike.

- **One-shot containers.** Always `docker run --rm`, in the foreground: the container is removed as soon as its command ends. Never keep one alive or reuse it: no `-d`, no `docker start`, `docker exec`, `docker attach` or `docker commit`. To run several commands, chain them in one `docker run … sh -c '…'`.
- **The code is mounted, never copied into an image**, for all the agents' work (build, tests, lock files):
  `docker run --rm -v "$PWD":/src -w /src --user "$(id -u):$(id -g)" -e CARGO_HOME=/src/target/cargo-home <toolchain-image> sh -c '…'`.
  Running as your own user keeps the generated files owned by you. Tool caches stay in a folder of the repository that git ignores.
- **Toolchain image** (the tools only, no code):
  - If the `Dockerfile` has a stage named `toolchain`, the image is that stage, tagged `sf-toolchain:<hash>` where `<hash>` is `sha256sum Dockerfile | cut -c1-12`. Build it only if it does not exist yet (`docker image inspect` fails): `docker build --target toolchain --label sf.factory=1 -t sf-toolchain:<hash> .`. Every agent and branch with the same `Dockerfile` reuses it.
  - Otherwise, it is the official image the build stage starts from (today the `FROM … AS build` image): nothing to build.
  - Extra tools a single command needs are installed inside its one-shot container, in the same `docker run`.
- **Product images** (`docker build --target test`, the runtime image): only to check what the requirements say about them. Tag them `sf-product-<branch>-$RANDOM` with `--label sf.factory=1`, and remove them with `docker rmi -f` as soon as the check is done, also when it fails.
- **Official base images** pulled from a registry are a download cache: keep them.
- **Cleanup.** An agent never removes a toolchain image. Before its final answer, it checks that it leaves no container (`docker ps -a`) and no product image (`docker images --filter label=sf.factory=1`), and says so in its report. After each pipeline, `factory.sh` removes every `sf.factory=1` image except the toolchain image of the `Dockerfile` on `master`.

## Requirement kinds

The factory, not the author, decides the kind of each requirement (the `req-checker` classifies it, the implementer writes `kind:` in its frontmatter):

- `feature`: an observable behaviour of the software. Implemented with code and automated tests.
- `constraint`: a technical rule on how the software is built (language, framework, tooling, conventions, layout). No feature code of its own: it is merged once recorded, and every later requirement must follow it. Its "How to test" steps that need application code are "not applicable yet" until such code exists.

A requirement that mixes both is rejected and must be split.

## Conventions

- **Requirement**: `requirements/<ID>-<short-title>.md`, following `requirements/_TEMPLATE.md`.
  - ID: `REQ-` followed by 3 digits (`REQ-001`). Unique, never reused.
  - A requirement is "merged" when its file is present on `master`. Until then it lives in its branch.
  - A merged requirement is changed with `/req-update <ID> <change>` (or edited by hand, then `factory.sh update <file>`). It keeps its ID and file name. It is never committed to `master` directly.
- **Branch**: `<ID>-<short-title-in-kebab-case>`, e.g. `REQ-001-email-login`. Only `[a-z0-9-]` after the ID.
- **Tests**: every requirement is covered by automated tests whenever possible. Their name contains the lowercase ID (e.g. `tests/req_001_*.py`, `req-001.test.ts`).
- **Main branch**: `master`.
- **Reports**: agent reports go to `.factory/reports/`. This folder is ignored by git.
- **Language**: everything in the repository (code, comments, requirements, commit messages, reports) is written in English.

## Tech stack

> Defined by the `kind: constraint` requirements merged on `master`: read them first. If none defines the stack, the first implemented feature picks it and describes it here, together with the command that runs the whole test suite.

- Language: Rust (stable toolchain, edition 2021 or newer), Cargo project at the repository root (REQ-001). Application code lives in `src/`, integration tests in `tests/`.
- Toolchain install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`
- Test command (whole suite): `cargo test`
- The build must produce no compiler warnings; `target/` is ignored by git.
- Containerized build (REQ-002): multi-stage `Dockerfile` + `.dockerignore` at the root (context = root); the host only needs Docker or Podman.
  - Tests in a container: `docker build --target test .` (runs `cargo test --locked`, fails on a failing test or a warning).
  - Runtime image: `docker build -t <name> .` (pinned minimal base, non-root user, binary copied to `/usr/local/bin/app` = `ENTRYPOINT`; the package must produce exactly one binary).
  - Base images are pinned to explicit tags (never `latest`); `Cargo.lock` is committed.

## Rules for all agents

- Never modify an existing requirement, except the one being implemented or updated.
- Never modify `master`: only the script moves it. No push, no rebase, no `reset --hard`. Single exception: the implementer, in conflict mode, may run `git merge master` **inside its own branch**.
- End the final answer with a `VERDICT: <VALUE>` line (values defined by each agent). The script reads this line.
