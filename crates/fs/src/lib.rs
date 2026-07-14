use std::path::PathBuf;

/// Manages the files
pub struct FileManager {
    pub current_path: Option<PathBuf>,
}

impl FileManager {
    pub fn new() -> Self {
        Self { current_path: None }
    }

    /// Opens file dialog and returns the selected path
    pub fn pick_file() -> Option<PathBuf> {
        rfd::FileDialog::new().set_title("Open File").pick_file()
    }

    /// Reads the full text content of a file
    pub fn read_file_at(path: &PathBuf) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    /// Returns the file size in bytes
    pub fn file_size(path: &PathBuf) -> Option<u64> {
        std::fs::metadata(path).ok().map(|m| m.len())
    }

    /// Opens save dialog
    pub fn save_as_dialog() -> Option<PathBuf> {
        rfd::FileDialog::new().set_title("Save File").save_file()
    }

    /// Saves content to path
    pub fn save_to_path(content: &str, path: &PathBuf) -> Result<(), std::io::Error> {
        // TODO: Add proper error handling
        std::fs::write(path, content)
    }
}
