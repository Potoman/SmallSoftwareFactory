---
id: REQ-002
title: Containerized build
kind: constraint
---

# REQ-002 — Containerized build

## Description

The software is built and tested inside a container. The host machine only needs an
OCI container engine able to build a `Dockerfile` (Docker or Podman). It does not need the
Rust toolchain. The choice of a specific engine is out of scope and will be set by a later
requirement.

Building and testing directly on the host with `cargo` (REQ-001) stays possible for anyone
who has the Rust toolchain: the container is an additional way, which needs no toolchain.

Rules:

- A single `Dockerfile` at the repository root (next to `Cargo.toml`), with a
  `.dockerignore` next to it. The build context is the repository root.
- The `Dockerfile` is a multi-stage build with at least these stages:
  - **build stage**: starts from an official Rust image pinned to an explicit version tag
    (never `latest`, never untagged) and compiles the software in release mode with
    `cargo build --release --locked`. Compiler warnings are errors in this stage
    (`RUSTFLAGS="-D warnings"`), consistently with REQ-001.
  - **`test` stage** (selected with `--target test`): runs the whole suite with
    `cargo test --locked`. If a test fails, the image build fails.
  - **runtime stage** (the last stage, built by default): a minimal base image pinned to
    an explicit tag, containing the compiled binary and only what it needs at runtime.
    It contains no Rust toolchain and no source code. It runs as a non-root user, and its
    `ENTRYPOINT` is the software's binary.
- `Cargo.lock` is committed, so `--locked` builds are reproducible.
- `.dockerignore` keeps the build context small and free of host state: at least
  `target/`, `.git/` and `.factory/` are excluded.
- `Dockerfile` and `.dockerignore` are created when this requirement is implemented, even if
  there is no application code yet. The image builds (and every "How to test" step that
  needs a container engine or application code) apply once application code exists.

Edge cases:

- A build from a fresh clone works with no manual step other than having a container
  engine installed.
- A failing test, or a compiler warning, makes the corresponding image build exit with a
  non-zero code.
- Building the runtime image does not run the tests, and building the `test` target does
  not produce the runtime image.

## How to test

In the steps below, `$ENGINE` is the installed container engine (`docker` or `podman`).

1. Run `test -f Dockerfile && test -f .dockerignore`.
   → Expected: exit code 0. Once application code exists, `git ls-files --error-unmatch Cargo.lock`
   must also exit with code 0.
2. Run `grep -cE '^FROM ' Dockerfile`.
   → Expected: a number greater than or equal to 2 (multi-stage build).
3. List the images of the `FROM` lines, ignoring `--platform=…` options, references to an
   earlier stage of the same file (`FROM build AS test`) and `scratch`:
   `awk 'toupper($1)=="FROM"{i=2; while($i ~ /^--/) i++; img=$i; if(toupper($(i+1))=="AS") st[tolower($(i+2))]=1; if(!(tolower(img) in st) && img!="scratch") print img}' Dockerfile`.
   → Expected: every listed image has an explicit tag other than `latest`, or a digest
   (`@sha256:`).
4. Run `grep -nE '^(target/?|/target/?|\.git/?|/\.git/?|\.factory/?|/\.factory/?)$' .dockerignore`.
   → Expected: one line for each of `target`, `.git` and `.factory`.
5. Run `$ENGINE build --target test -t req002-test .`
   → Expected: exit code 0, and the build output shows the test run with every test `ok`.
6. In a temporary copy of the repository, add `tests/req_002_must_fail.rs` containing
   `#[test] fn must_fail() { panic!("expected failure"); }`, then run
   `$ENGINE build --target test .` in that copy.
   → Expected: non-zero exit code.
7. Run `$ENGINE build -t req002-app .`
   → Expected: exit code 0.
8. Run `$ENGINE image inspect req002-app --format '{{.Config.User}}'`.
   → Expected: a non-empty value that is neither `root` nor `0`.
9. Run `$ENGINE image inspect req002-app --format '{{json .Config.Entrypoint}}'`.
   → Expected: a non-empty list whose first element is the path of the software's binary.
10. Run `$ENGINE run --rm --entrypoint cargo req002-app --version`.
    → Expected: non-zero exit code (no Rust toolchain in the runtime image).
