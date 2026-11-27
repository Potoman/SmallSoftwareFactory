//! Command-line parsing (REQ-003). The program defines no option and no argument yet.

use std::ffi::OsString;
use std::fmt;

/// A command line the program does not accept.
#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    /// An option (an argument starting with `-`) the program does not define.
    UnknownOption(String),
    /// A positional argument, while the program takes none.
    UnexpectedArgument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::UnknownOption(opt) => write!(f, "unknown option '{opt}'"),
            CliError::UnexpectedArgument(arg) => write!(f, "unexpected argument '{arg}'"),
        }
    }
}

/// Checks the command-line arguments (without the program name).
pub fn parse<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
{
    match args.into_iter().next() {
        None => Ok(()),
        Some(arg) => {
            let text = arg.to_string_lossy().into_owned();
            if text.starts_with('-') && text != "-" {
                Err(CliError::UnknownOption(text))
            } else {
                Err(CliError::UnexpectedArgument(text))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn req_003_no_argument_is_accepted() {
        assert_eq!(parse(args(&[])), Ok(()));
    }

    #[test]
    fn req_003_unknown_long_option_is_rejected() {
        assert_eq!(
            parse(args(&["--req-003-unknown-option"])),
            Err(CliError::UnknownOption("--req-003-unknown-option".into()))
        );
    }

    #[test]
    fn req_003_unknown_short_option_is_rejected() {
        assert_eq!(parse(args(&["-x"])), Err(CliError::UnknownOption("-x".into())));
    }

    #[test]
    fn req_003_positional_argument_is_rejected() {
        assert_eq!(
            parse(args(&["file"])),
            Err(CliError::UnexpectedArgument("file".into()))
        );
    }

    #[test]
    fn req_003_error_message_names_the_option() {
        let msg = CliError::UnknownOption("--nope".into()).to_string();
        assert!(msg.contains("--nope"), "{msg}");
    }
}
