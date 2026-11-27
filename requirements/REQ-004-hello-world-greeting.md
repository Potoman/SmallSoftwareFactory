---
id: REQ-004
title: Hello World greeting at startup
kind: feature
---

# REQ-004 — Hello World greeting at startup

## Description

When the program starts, the first thing it does is print a greeting on standard output.

Rules:

- The greeting is exactly `Hello World !` (capital `H` and `W`, one space before the
  exclamation mark), followed by a single line feed (`\n`). In bytes: the 14 bytes
  `Hello World !\n`, with no carriage return, no leading or trailing spaces, no colors.
- It is printed on **standard output**, exactly once per run, before any other output of
  the program.
- It is printed at every start, whatever the command line: also when the command line is
  then rejected (REQ-003). In that case standard output holds only the greeting, and the
  error message goes to standard error as before.
- Nothing else is printed on standard output for now.
- The rest of REQ-003 is unchanged: after the greeting the program does its work and exits
  with code `0` when the command line is valid.

Edge cases:

- Standard output is always considered open. The program does not detect or handle a
  closed standard output: its behaviour in that case is not specified.
- The greeting is written in full and flushed before the program exits, even when standard
  output is a pipe or a file (not a terminal).

## How to test

`APP` is the absolute path of the software's binary, run in the environment it was built
for (on the host, or inside the `rust` build container of REQ-002).

1. Run `"$APP" </dev/null > out.txt; echo "exit=$?"; od -c out.txt`.
   → Expected: `exit=0`, and `od` shows exactly `H e l l o   W o r l d   ! \n` (14 bytes,
   `wc -c < out.txt` prints `14`).
2. Run `"$APP" </dev/null | cat`.
   → Expected: the output is exactly the line `Hello World !`.
3. Run `"$APP" --req-004-unknown-option </dev/null > out.txt 2> err.txt; echo "exit=$?"`.
   → Expected: a non-zero exit code, `out.txt` contains exactly `Hello World !\n`, and
   `err.txt` holds the error message (at least one line) and no greeting.
4. Build the runtime image of REQ-002 (`$ENGINE build -t req004-app .`) and run
   `$ENGINE run --rm req004-app`.
   → Expected: exit code `0`, and the output is exactly the line `Hello World !`.
5. The automated tests carrying `req_004` in their name cover steps 1 to 3 and run with the
   whole suite (`cargo test`).
   → Expected: they pass.
