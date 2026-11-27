//! REQ-003: the program runs to completion and exits.
//!
//! Every test gives up with a failure (and kills the program) instead of hanging when the
//! program does not stop within the expected time (step 6). The tests run one at a time,
//! so that the "no process left" check (step 5) only sees processes of the current test.

use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const APP: &str = env!("CARGO_BIN_EXE_software-factory");

/// Time the program has to stop by itself or after a signal.
const STOP_LIMIT: Duration = Duration::from_secs(5);

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Waits for `child` at most `limit`. On timeout, kills it and fails the test.
fn wait_or_fail(child: &mut Child, limit: Duration, what: &str) -> (ExitStatus, Duration) {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            return (status, start.elapsed());
        }
        if start.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what}: still running after {limit:?}, killed");
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

/// Pids of the running processes whose `argv[0]` is the program (`pgrep -f "^$APP( |$)"`).
fn running_app_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return pids;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        // A zombie has an empty cmdline: it is no longer running.
        let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let argv0 = cmdline.split(|b| *b == 0).next().unwrap_or_default();
        if !argv0.is_empty() && Path::new(std::str::from_utf8(argv0).unwrap_or("")) == Path::new(APP) {
            pids.push(pid);
        }
    }
    pids
}

/// Step 5: no process of the program is left.
fn assert_nothing_left() {
    let pids = running_app_pids();
    assert!(pids.is_empty(), "processes of {APP} still running: {pids:?}");
}

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;

fn send_signal(child: &Child, sig: i32) {
    // SAFETY: plain system call on the pid of a child we have not reaped yet.
    let rc = unsafe { kill(child.id() as i32, sig) };
    assert_eq!(rc, 0, "kill({}, {sig}) failed", child.id());
}

/// Steps 3 and 4: start the program, signal it right away, it stops within 5 seconds.
fn interrupt(sig: i32, interrupted_code: i32) {
    let _guard = serial();
    let mut child = Command::new(APP)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("cannot start the program");
    send_signal(&child, sig);
    let (status, _) = wait_or_fail(&mut child, STOP_LIMIT, "after the signal");
    let code = shell_code(status);
    assert!(
        code == 0 || code == interrupted_code,
        "expected exit 0 or {interrupted_code}, got {code} ({status:?})"
    );
    assert_nothing_left();
}

#[test]
fn req_003_step1_exits_0_with_stdin_pipe_left_open() {
    let _guard = serial();
    let start = Instant::now();
    let mut child = Command::new(APP)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("cannot start the program");
    // Keep the write end of the pipe open for the whole wait.
    let stdin = child.stdin.take();
    let (status, _) = wait_or_fail(&mut child, Duration::from_secs(30), "stdin left open");
    let elapsed = start.elapsed();
    drop(stdin);
    assert_eq!(status.code(), Some(0), "status: {status:?}");
    assert!(elapsed < STOP_LIMIT, "took {elapsed:?}");
    assert_nothing_left();
}

#[test]
fn req_003_step2_unknown_option_fails_with_message_on_stderr() {
    let _guard = serial();
    let mut child = Command::new(APP)
        .arg("--req-003-unknown-option")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot start the program");
    let mut stderr = child.stderr.take().expect("stderr");
    let reader = thread::spawn(move || {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut stderr, &mut text).expect("read stderr");
        text
    });
    let (status, _) = wait_or_fail(&mut child, STOP_LIMIT, "unknown option");
    let err = reader.join().expect("stderr reader");
    let code = shell_code(status);
    assert_ne!(code, 0, "an unknown option must fail");
    assert_ne!(code, 124);
    assert!(err.lines().count() >= 1, "no message on stderr");
    assert!(err.contains("--req-003-unknown-option"), "stderr: {err}");
    assert_nothing_left();
}

#[test]
fn req_003_step3_sigterm_stops_with_0_or_143() {
    interrupt(SIGTERM, 143);
}

#[test]
fn req_003_step4_sigint_stops_with_0_or_130() {
    interrupt(SIGINT, 130);
}

#[test]
fn req_003_step5_no_process_left_after_normal_run() {
    let _guard = serial();
    for _ in 0..5 {
        let mut child = Command::new(APP)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("cannot start the program");
        let (status, _) = wait_or_fail(&mut child, STOP_LIMIT, "normal run");
        assert_eq!(status.code(), Some(0));
        assert_nothing_left();
    }
}

#[test]
fn req_003_step6_running_processes_are_detected() {
    // Guards step 5: the detection must see a running program, otherwise
    // `assert_nothing_left` would pass vacuously. A `sleep` started with the program's
    // path as argv[0] stands in for a program that does not stop.
    use std::os::unix::process::CommandExt;
    let _guard = serial();
    let sleep = ["/bin/sleep", "/usr/bin/sleep"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .expect("sleep not found");
    let mut child = Command::new(sleep)
        .arg0(APP)
        .arg("30")
        .stdin(Stdio::null())
        .spawn()
        .expect("cannot start sleep");
    // Retried for a short while: `/proc` may lag right after the spawn.
    let start = Instant::now();
    let seen = loop {
        if running_app_pids().contains(&child.id()) {
            break true;
        }
        if start.elapsed() >= Duration::from_secs(2) {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let _ = child.kill();
    let _ = child.wait();
    assert!(seen, "a running process with argv[0] = {APP} was not detected");
    assert_nothing_left();
}
