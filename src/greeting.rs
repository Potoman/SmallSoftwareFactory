//! Startup greeting (REQ-004).
//!
//! The program prints exactly `Hello World !\n` on standard output, once, before anything
//! else. If standard output cannot be written, the greeting fails with an error instead of
//! panicking, and the caller turns it into a failure in the sense of REQ-003.
//!
//! A closed standard output needs special care: before `main`, the Rust runtime reopens a
//! closed standard file descriptor on `/dev/null`, and `std::io::stdout` silently ignores
//! `EBADF`. Writing the greeting would then "succeed" without anything being printed. So
//! the state of file descriptor 1 is recorded by a constructor that runs before the Rust
//! runtime starts (see [`startup`]).

use std::io::{self, Write};

/// The greeting, byte for byte.
pub const GREETING: &[u8] = b"Hello World !\n";

/// Writes the greeting to `out` and flushes it.
pub fn write_to<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(GREETING)?;
    out.flush()
}

/// Prints the greeting on standard output and flushes it.
pub fn print() -> io::Result<()> {
    if startup::stdout_was_closed() {
        return Err(io::Error::other("standard output is closed"));
    }
    let mut out = io::stdout().lock();
    write_to(&mut out)
}

#[cfg(target_os = "linux")]
mod startup {
    use std::os::raw::c_int;
    use std::sync::atomic::{AtomicBool, Ordering};

    const STDOUT_FILENO: c_int = 1;
    const F_GETFD: c_int = 1;

    extern "C" {
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    }

    static STDOUT_CLOSED: AtomicBool = AtomicBool::new(false);

    /// Runs from `.init_array`, before the Rust runtime reopens closed standard
    /// file descriptors on `/dev/null`.
    extern "C" fn record_stdout_state() {
        // SAFETY: `fcntl(F_GETFD)` only queries the descriptor flags; it fails (with
        // `EBADF`) when the descriptor is not open.
        let closed = unsafe { fcntl(STDOUT_FILENO, F_GETFD) } == -1;
        STDOUT_CLOSED.store(closed, Ordering::SeqCst);
    }

    #[used]
    #[link_section = ".init_array"]
    static RECORD_STDOUT_STATE: extern "C" fn() = record_stdout_state;

    /// Whether standard output was closed when the process started.
    pub fn stdout_was_closed() -> bool {
        STDOUT_CLOSED.load(Ordering::SeqCst)
    }
}

#[cfg(not(target_os = "linux"))]
mod startup {
    /// No detection on this platform: write errors are still reported.
    pub fn stdout_was_closed() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn req_004_greeting_is_exactly_14_bytes() {
        assert_eq!(GREETING, b"Hello World !\n");
        assert_eq!(GREETING.len(), 14);
    }

    #[test]
    fn req_004_write_to_writes_the_greeting_once() {
        let mut out = Vec::new();
        write_to(&mut out).expect("write");
        assert_eq!(out, GREETING);
    }

    #[test]
    fn req_004_write_error_is_reported() {
        assert!(write_to(&mut FailingWriter).is_err());
    }
}
