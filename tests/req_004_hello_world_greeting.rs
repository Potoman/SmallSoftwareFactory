//! REQ-004: the program prints `Hello World !\n` on standard output at startup.
//!
//! Every test gives up with a failure (and kills the program) instead of hanging when the
//! program does not stop in time.

use std::fs::File;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const APP: &str = env!("CARGO_BIN_EXE_software-factory");

const GREETING: &[u8] = b"Hello World !\n";

/// Time the program has to stop by itself.
const STOP_LIMIT: Duration = Duration::from_secs(5);

/// Waits for `child` at most `STOP_LIMIT`. On timeout, kills it and fails the test.
fn wait_or_fail(child: &mut Child, what: &str) -> ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            return status;
        }
        if start.elapsed() >= STOP_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what}: still running after {STOP_LIMIT:?}, killed");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Exit code as a shell reports it: the code itself, or 128 + signal number.
fn shell_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => code,
        (None, Some(sig)) => 128 + sig,
        (None, None) => panic!("unexpected exit status {status:?}"),
    }
}

/// A fresh, empty directory for the files of one test.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Runs the program with `args`, stdin from `/dev/null`, stdout and stderr redirected to
/// files (like `"$APP" </dev/null >out.txt 2>err.txt`). Returns the exit code, stdout and
/// stderr.
fn run_to_files(test: &str, args: &[&str]) -> (i32, Vec<u8>, String) {
    let dir = scratch_dir(test);
    let out_path = dir.join("out.txt");
    let err_path = dir.join("err.txt");
    let mut child = Command::new(APP)
        .args(args)
        .stdin(Stdio::null())
        .stdout(File::create(&out_path).expect("create out.txt"))
        .stderr(File::create(&err_path).expect("create err.txt"))
        .spawn()
        .expect("cannot start the program");
    let status = wait_or_fail(&mut child, test);
    let out = std::fs::read(&out_path).expect("read out.txt");
    let err = std::fs::read_to_string(&err_path).expect("read err.txt");
    (shell_code(status), out, err)
}

#[test]
fn req_004_step1_greeting_written_to_a_file() {
    let (code, out, err) = run_to_files("req_004_step1", &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out.len(), 14, "stdout: {:?}", String::from_utf8_lossy(&out));
    assert_eq!(out, GREETING);
}

#[test]
fn req_004_step2_greeting_written_to_a_pipe() {
    let child = Command::new(APP)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot start the program");
    let start = Instant::now();
    let output = child.wait_with_output().expect("wait for the program");
    assert!(start.elapsed() < STOP_LIMIT, "took {:?}", start.elapsed());
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, GREETING);
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(text.lines().collect::<Vec<_>>(), ["Hello World !"]);
}

#[test]
fn req_004_step3_greeting_printed_even_when_command_line_is_rejected() {
    let (code, out, err) = run_to_files("req_004_step3", &["--req-004-unknown-option"]);
    assert_ne!(code, 0, "an unknown option must fail");
    assert_eq!(out, GREETING, "stdout: {:?}", String::from_utf8_lossy(&out));
    assert!(err.lines().count() >= 1, "no message on stderr");
    assert!(err.contains("--req-004-unknown-option"), "stderr: {err}");
    assert!(!err.contains("Hello World"), "greeting on stderr: {err}");
}
