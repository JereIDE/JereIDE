use std::path::{Path, PathBuf};

/// Opens file dialog and returns path!!!
pub fn pick_file() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Open File").pick_file()
}

/// Actually the same, but this time it's a directory.
pub fn pick_directory() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Open Project")
        .pick_folder()
}

/// Reads the full text content of a file...
pub fn read_file_at(path: &PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Returns the file size in bytes I think
pub fn file_size(path: &PathBuf) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

/// Opens save dialog
pub fn save_as_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Save File").save_file()
}

/// Saves content to path
pub fn save_to_path(content: &str, path: &PathBuf) -> Result<(), std::io::Error> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    let temp_path = dir.join(format!(".{}.jereide-tmp", file_name.to_string_lossy()));

    let result = (|| {
        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
    pub is_symlink: bool,
}

pub fn list_directory(path: &Path) -> Result<Vec<DirectoryEntry>, std::io::Error> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        entries.push(DirectoryEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_directory: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }
    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_directory_sorts_directories_first() {
        let dir = std::env::temp_dir().join(format!("jereide_fs_ls_sort_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::File::create(dir.join("b.txt")).unwrap();
        std::fs::create_dir(dir.join("a_dir")).unwrap();

        let entries = list_directory(&dir).unwrap();
        assert!(entries[0].is_directory);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
