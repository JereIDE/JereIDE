use std::path::PathBuf;

pub fn data_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("data");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("data");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}
