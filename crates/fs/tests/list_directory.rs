use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use jereide_fs::list_directory;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jereide_fs_{}_{}_{}",
        tag,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
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
