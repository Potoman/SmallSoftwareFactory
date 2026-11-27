---
id: REQ-001
title: Rust tech stack
kind: constraint
---

# REQ-001 — Rust tech stack

## Description

The software is written in Rust. Every later requirement builds on this choice.

Rules:

- The repository root is a Cargo project: a `Cargo.toml` at the root, using the stable Rust
  toolchain and the 2021 edition (or newer).
- All application code is Rust, under `src/` (and `tests/` for integration tests). No other
  programming language is used for application code.
- Files that are not application code are allowed in other formats: the factory itself
  (`scripts/`, `.claude/`, `CLAUDE.md`, `requirements/`), git configuration files, and
  documentation under `docs/` or a root `README*`.
- The whole test suite runs with a single command from the repository root: `cargo test`.
- The project builds without compiler warnings.
- Build output (`target/`) is ignored by git.

Edge cases:

- The Rust toolchain is installed with `rustup` (stable channel):
  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`.
- Running `cargo test` on a fresh clone, with no prior build, must work without any
  manual step other than having the toolchain installed.

## How to test

1. Run `cargo --version` and `rustc --version`.
   → Expected: both print a version; exit code 0.
2. From the repository root, run `cargo build`.
   → Expected: exit code 0, and the output contains no line starting with `warning:`.
3. From the repository root, run `cargo test`.
   → Expected: exit code 0, and every test reported is `ok`.
4. Run `git ls-files | grep -vE '^(scripts/|\.claude/|docs/|requirements/|CLAUDE\.md$|README|\.gitattributes$|\.gitignore$|Cargo\.(toml|lock)$)'`.
   → Expected: every listed file ends in `.rs` (no application code in another language).
5. Run `git check-ignore target/`.
   → Expected: prints `target/`; exit code 0.
