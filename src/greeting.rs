//! Startup greeting (REQ-004).
//!
//! The program prints exactly `Hello World !\n` on standard output, once, before anything
//! else, and flushes it. Standard output is assumed to be open; a write error is still
//! returned instead of panicking, and the caller reports it on standard error.

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
    let mut out = io::stdout().lock();
    write_to(&mut out)
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
