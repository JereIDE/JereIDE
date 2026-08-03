use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use jereide_backend::run_command;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jereide_backend_{}_{}_{}",
        tag,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn run_command_echo_success() {
    let out = run_command("echo \"hello from shell\"", None).unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.trim(), "hello from shell");
    assert_eq!(out.stderr, "");
}

#[test]
#[cfg(not(windows))]
fn run_command_captures_stderr_separately() {
    let out = run_command("echo to-stdout; echo to-stderr >&2", None).unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.trim(), "to-stdout");
    assert_eq!(out.stderr.trim(), "to-stderr");
}

#[test]
#[cfg(not(windows))]
fn run_command_reports_nonzero_exit_code() {
    let out = run_command("exit 3", None).unwrap();
    assert_eq!(out.exit_code, 3);
    assert_eq!(out.stdout, "");
}

#[test]
#[cfg(not(windows))]
fn run_command_runs_in_cwd() {
    let dir = unique_temp_dir("cwd");
    let canonical = std::fs::canonicalize(&dir).unwrap();
    let out = run_command("pwd", Some(&canonical)).unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.trim(), canonical.to_str().unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_command_invalid_cwd_is_error() {
    let missing = unique_temp_dir("missing");
    std::fs::remove_dir_all(&missing).unwrap();
    assert!(run_command("echo hi", Some(&missing)).is_err());
}
