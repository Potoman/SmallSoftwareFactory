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
  programming language is used for application code: no source file in another programming
  language (Python, JavaScript, TypeScript, Go, Java, Kotlin, C, C++, C#, Ruby, PHP, Swift…).
- This rule is about application code only. Every other file is allowed, in any format, as
  long as it does not implement application logic:
  - the factory itself (`scripts/`, `.claude/`, `CLAUDE.md`, `requirements/`);
  - environment, build and deployment configuration: container files (`Dockerfile`,
    `.dockerignore`, `compose.yaml`…), Cargo and Rust tool configuration (`.cargo/`,
    `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`…), CI configuration, environment
    files (`.env.example`…), application configuration files (TOML, YAML, JSON…);
  - shell scripts used to build, run or deploy the software (for example a container
    entrypoint), as long as the application logic stays in Rust;
  - test data and fixtures, documentation (`docs/`, `README*`, `LICENSE*`), git configuration.
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
4. Run `git ls-files | grep -vE '^(scripts/|\.claude/)' | grep -iE '\.(py|pyw|js|mjs|cjs|jsx|ts|tsx|go|java|kt|kts|scala|groovy|c|h|cc|cpp|cxx|hpp|hh|cs|fs|vb|rb|php|swift|m|mm|dart|lua|pl|pm|r|jl|ex|exs|erl|hs|clj|zig|nim)$'`.
   → Expected: no output (no source file of another programming language outside the
   factory). Files such as a `Dockerfile`, `.dockerignore`, `*.toml`, `*.yaml`, `*.json`,
   `*.env.example`, `*.md` or `LICENSE` are allowed and not listed.
5. Run `git check-ignore target/`.
   → Expected: prints `target/`; exit code 0.
