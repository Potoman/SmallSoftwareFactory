---
id: REQ-003
title: Run to completion and exit
kind: feature
---

# REQ-003 — Run to completion and exit

## Description

The software is a program that runs, does its work, then stops by itself. It is not a
service or a daemon: it never keeps running, waiting or looping once its work is done.
What the work is will be defined by later requirements; until then, the work is empty and
the program stops right away.

Rules:

- **Success**: when its work is done, the program exits with code `0`.
- **Failure**: when its work cannot be done, the program stops, prints an error message on
  standard error (stderr), and exits with a non-zero code. It does not hang or retry
  forever.
  - An unknown command-line option (an argument starting with `--` that the program does
    not define) is a failure.
- **Interruption**: when it receives `SIGINT` (Ctrl+C) or `SIGTERM` (for example from
  `docker stop`) before its work is done, it stops within 5 seconds and exits with code
  `130` for `SIGINT` or `143` for `SIGTERM`.
- **Nothing left behind**: once the program has exited, none of its processes (children
  included) is still running.

Edge cases:

- A signal received after the work is done, while the program is already exiting, does
  not change a success into a failure if the exit code was already decided.
- The program does not wait for standard input to be closed before stopping: it must stop
  even when started with standard input connected to a pipe that stays open.

## How to test

The steps use bash. `APP` is the absolute path of the software's binary, run in the
environment it was built for: on the host after `cargo build --release` (REQ-001), or inside
the `rust` build container of REQ-002 when the host has no Rust toolchain. In steps 3 and 4,
the program is started through `env --default-signal=INT,TERM`, so that it does not inherit
signals ignored by the shell that starts it in the background.

1. Run `start=$(date +%s.%N); timeout 30 "$APP" < <(sleep 60); code=$?; echo "exit=$code after $(echo "$(date +%s.%N) - $start" | bc)s"`
   (standard input is a pipe that stays open; the shell does not wait for `sleep`).
   → Expected: `exit=0`, after less than 5 seconds (`124` would mean it did not stop by
   itself).
2. Run `"$APP" --req-003-unknown-option </dev/null 2>err.txt; echo "exit=$?"; wc -l < err.txt`.
   → Expected: a non-zero exit code other than `124`, and a line count of at least 1.
3. Start the program in the background, send it `SIGTERM` right away, and wait at most
   5 seconds for it to stop:
   `env --default-signal=INT,TERM "$APP" </dev/null & pid=$!; kill -TERM $pid; for i in $(seq 50); do kill -0 $pid 2>/dev/null || break; sleep 0.1; done; if kill -0 $pid 2>/dev/null; then echo "still running"; kill -KILL $pid; fi; wait $pid; echo "exit=$?"`.
   → Expected: no `still running` line, and `exit=0` (its work was already done) or
   `exit=143`.
4. Same as step 3 with `kill -INT`.
   → Expected: no `still running` line, and `exit=0` or `exit=130`.
5. After each of the steps above, run `pgrep -f "^$APP( |$)"`.
   → Expected: no output (no process of the program left).
6. The automated tests carrying `req_003` in their name cover steps 1 to 5, run with the
   whole suite (`cargo test`), and give up with a failure, instead of hanging, if the
   program does not stop within the expected time.
   → Expected: they pass.
