use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use jereide_backend::{list_directory, run_command};

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

#[test]
fn list_directory_returns_known_entries() {
    let entries = list_directory(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    assert!(entries
        .iter()
        .any(|e| e.name == "Cargo.toml" && !e.is_directory));
    assert!(entries.iter().any(|e| e.name == "src" && e.is_directory));
}

#[test]
fn list_directory_skips_dot_entries() {
    let dir = unique_temp_dir("dots");
    std::fs::File::create(dir.join("a.txt")).unwrap();
    std::fs::create_dir(dir.join("sub")).unwrap();

    let entries = list_directory(&dir).unwrap();
    assert!(!entries.iter().any(|e| e.name == "." || e.name == ".."));
    assert!(entries.iter().any(|e| e.name == "a.txt"));
    assert!(entries.iter().any(|e| e.name == "sub"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_directory_preserves_names_with_spaces() {
    let dir = unique_temp_dir("spaces");
    std::fs::File::create(dir.join("my file.txt")).unwrap();

    let entries = list_directory(&dir).unwrap();
    assert!(entries.iter().any(|e| e.name == "my file.txt"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_directory_includes_hidden_files() {
    let dir = unique_temp_dir("hidden");
    std::fs::File::create(dir.join(".hidden")).unwrap();

    let entries = list_directory(&dir).unwrap();
    assert!(entries.iter().any(|e| e.name == ".hidden"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn list_directory_marks_symlinks() {
    let dir = unique_temp_dir("symlink");
    std::fs::File::create(dir.join("target.txt")).unwrap();
    std::os::unix::fs::symlink(dir.join("target.txt"), dir.join("link.txt")).unwrap();

    let entries = list_directory(&dir).unwrap();
    assert!(entries.iter().any(|e| e.name == "link.txt" && e.is_symlink));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_directory_missing_dir_is_error() {
    let missing = unique_temp_dir("missing_dir");
    std::fs::remove_dir_all(&missing).unwrap();
    assert!(list_directory(&missing).is_err());
}
