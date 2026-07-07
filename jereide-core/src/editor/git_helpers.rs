/// Pure git helper functions extracted from `main_loop::run()`.
///
/// These are blocking helpers used by the git-blame and git-log overlays.
/// They live here so `main_loop.rs` doesn't need nested `fn` definitions
/// that slow incremental compilation.

/// Trivial days-since-epoch to (year, month, day) for blame dates.
pub(crate) fn epoch_to_ymd(days_since_epoch: i64) -> (i64, i64, i64) {
    // Algorithm from Howard Hinnant's civil_from_days (public domain).
    let z = days_since_epoch + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Run `git status --porcelain=v1` and return (code, path, display) tuples.
pub(crate) fn run_git_status(root: &str) -> Vec<(String, String, String)> {
    let Ok(output) = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let code = line[..2].trim().to_string();
            let path = line[3..].trim().to_string();
            let display = format!("[{code}] {path}");
            Some((code, path, display))
        })
        .collect()
}

/// Run `git blame --porcelain` and return one summary string per line.
pub(crate) fn run_git_blame(file_path: &str) -> Vec<String> {
    let Ok(output) = std::process::Command::new("git")
        .args(["blame", "--porcelain", "--", file_path])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    // Porcelain format: blocks of header lines followed by a tab-prefixed
    // source line. Each block starts with a 40-char hash. We collect
    // author + author-time for each block, then build a compact summary.
    let text = String::from_utf8_lossy(&output.stdout);
    let mut result: Vec<String> = Vec::new();
    let mut hash = String::new();
    let mut author = String::new();
    let mut date = String::new();
    for line in text.lines() {
        // Block header: 40-char hash followed by line numbers.
        if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
            hash = line[..8].to_string();
        } else if let Some(a) = line.strip_prefix("author ") {
            author = a.to_string();
        } else if let Some(ts) = line.strip_prefix("author-time ") {
            if let Ok(epoch) = ts.parse::<i64>() {
                let days = epoch / 86400;
                let (y, m, d) = epoch_to_ymd(days);
                date = format!("{y:04}-{m:02}-{d:02}");
            }
        } else if line.starts_with('\t') {
            // End of block — emit the summary for this source line.
            let short_author: String = author.chars().take(20).collect();
            result.push(format!("{hash}  {short_author:<20}  {date}"));
            author.clear();
            date.clear();
            hash.clear();
        }
    }
    result
}

/// Run `git log --oneline` for a file and return (hash, date, message).
pub(crate) fn run_git_log(file_path: &str) -> Vec<(String, String, String)> {
    let Ok(output) = std::process::Command::new("git")
        .args(["log", "--format=%h|%as|%s", "-50", "--", file_path])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '|');
            let hash = parts.next()?.to_string();
            let date = parts.next()?.to_string();
            let msg = parts.next().unwrap_or("").to_string();
            Some((hash, date, msg))
        })
        .collect()
}
