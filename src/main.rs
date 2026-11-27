//! SoftwareFactory: a program that runs its work to completion, then exits (REQ-003).
//!
//! Exit codes:
//! - `0`: the work is done;
//! - `2`: invalid command line (unknown option or unexpected argument), with a message on stderr;
//! - `130` / `143`: interrupted by `SIGINT` / `SIGTERM` before the work was done.

mod cli;
mod signals;

use std::process::ExitCode;

/// Exit code for a command line the program does not accept.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    // Installed first, so that an early SIGINT/SIGTERM stops the program with 130/143.
    signals::install();

    let code = match cli::parse(std::env::args_os().skip(1)) {
        Ok(()) => {
            run();
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(EXIT_USAGE)
        }
    };

    // The exit code is decided: a signal received from now on does not change it.
    signals::mark_done();
    code
}

/// The program's work. Empty until a later requirement defines it: the program stops
/// right away. It never waits on standard input.
fn run() {}
