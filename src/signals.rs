//! Interruption by `SIGINT` / `SIGTERM` (REQ-003).
//!
//! Until the exit code is decided, `SIGINT` and `SIGTERM` stop the program at once with
//! exit code `128 + signal number` (`130` and `143`). The handler only calls
//! async-signal-safe operations (an atomic load and `_exit`). Installing the handlers
//! explicitly also makes the program stop on `docker stop` when it runs as PID 1, where
//! the kernel ignores signals that have no handler.
//!
//! Once [`mark_done`] has been called, these signals are ignored, so that a signal
//! received while the program is already exiting does not turn a success into a failure.

/// Exit code of a program stopped by signal `signal`, following the shell convention.
pub fn exit_code_for_signal(signal: i32) -> i32 {
    128 + signal
}

#[cfg(unix)]
mod imp {
    use std::os::raw::c_int;
    use std::sync::atomic::{AtomicBool, Ordering};

    pub const SIGINT: c_int = 2;
    pub const SIGTERM: c_int = 15;

    /// `SIG_ERR`, returned by `signal` on failure.
    const SIG_ERR: usize = usize::MAX;

    extern "C" {
        fn signal(signum: c_int, handler: usize) -> usize;
        fn _exit(status: c_int) -> !;
    }

    static DONE: AtomicBool = AtomicBool::new(false);

    extern "C" fn on_signal(sig: c_int) {
        if !DONE.load(Ordering::SeqCst) {
            // SAFETY: `_exit` is async-signal-safe and never returns.
            unsafe { _exit(super::exit_code_for_signal(sig)) }
        }
    }

    pub fn install() {
        let handler = on_signal as extern "C" fn(c_int) as usize;
        for sig in [SIGINT, SIGTERM] {
            // SAFETY: `on_signal` is an `extern "C" fn(c_int)` that only performs
            // async-signal-safe operations.
            let previous = unsafe { signal(sig, handler) };
            if previous == SIG_ERR {
                eprintln!("warning: cannot install the handler for signal {sig}");
            }
        }
    }

    pub fn mark_done() {
        DONE.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn is_done() -> bool {
        DONE.load(Ordering::SeqCst)
    }
}

#[cfg(not(unix))]
mod imp {
    pub fn install() {}
    pub fn mark_done() {}
}

/// Installs the `SIGINT` and `SIGTERM` handlers.
pub fn install() {
    imp::install();
}

/// Records that the exit code is decided: `SIGINT` and `SIGTERM` no longer change it.
pub fn mark_done() {
    imp::mark_done();
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn req_003_sigint_maps_to_130() {
        assert_eq!(exit_code_for_signal(imp::SIGINT), 130);
    }

    #[test]
    fn req_003_sigterm_maps_to_143() {
        assert_eq!(exit_code_for_signal(imp::SIGTERM), 143);
    }

    #[test]
    fn req_003_mark_done_is_recorded() {
        mark_done();
        assert!(imp::is_done());
    }
}
