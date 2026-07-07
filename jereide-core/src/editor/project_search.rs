/// Project-wide search and replace helpers extracted from `main_loop::run()`.
///
/// These are self-contained functions with a static cache for the async search.
/// They live here so `main_loop.rs` doesn't need nested `fn` definitions
/// that slow incremental compilation.

/// Run grep across the project, returning (path, line_number, line_text) tuples.
/// Blocking project-wide grep. Runs on a worker thread spawned by
/// `run_project_search`; do not call directly from the render loop.
pub(crate) fn project_search_blocking(
    query: &str,
    root: &str,
    use_regex: bool,
    whole_word: bool,
    case_insensitive: bool,
) -> Vec<(String, usize, String)> {
    let mut args = vec!["-rn".to_string()];
    if case_insensitive {
        args.push("-i".to_string());
    }
    if !use_regex {
        args.push("-F".to_string()); // fixed string (literal)
    }
    if whole_word {
        args.push("-w".to_string());
    }
    for pat in &[
        "--include=*.rs",
        "--include=*.toml",
        "--include=*.json",
        "--include=*.md",
        "--include=*.txt",
        "--include=*.js",
        "--include=*.ts",
        "--include=*.py",
        "--include=*.go",
        "--include=*.c",
        "--include=*.h",
        "--include=*.cpp",
        "--include=*.java",
    ] {
        args.push(pat.to_string());
    }
    args.push(query.to_string());
    args.push(root.to_string());
    let output = std::process::Command::new("grep").args(&args).output();
    let Ok(out) = output else {
        return Vec::new();
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut results = Vec::new();
    for line in stdout.lines().take(100) {
        // Format: path:line_num:text
        let mut parts = line.splitn(3, ':');
        let Some(path) = parts.next() else { continue };
        let Some(num_str) = parts.next() else {
            continue;
        };
        let Some(text) = parts.next() else { continue };
        let Ok(line_num) = num_str.parse::<usize>() else {
            continue;
        };
        results.push((path.to_string(), line_num, text.trim().to_string()));
    }
    results
}

/// Non-blocking project-wide search. Returns the most recently completed
/// results immediately and runs grep on a worker thread, so typing in the
/// find box never blocks the render loop; the per-frame pump applies fresh
/// results for the active panel when they land.
pub(crate) fn run_project_search(
    query: &str,
    root: &str,
    use_regex: bool,
    whole_word: bool,
    case_insensitive: bool,
) -> Vec<(String, usize, String)> {
    type Hit = (String, usize, String);
    type Key = (String, String, bool, bool, bool);
    struct SearchCache {
        ready: std::collections::HashMap<Key, Vec<Hit>>,
        order: std::collections::VecDeque<Key>,
        inflight: std::collections::HashSet<Key>,
        most_recent: Vec<Hit>,
    }
    if query.len() < 2 {
        return Vec::new();
    }
    static CACHE: std::sync::LazyLock<parking_lot::Mutex<SearchCache>> =
        std::sync::LazyLock::new(|| {
            parking_lot::Mutex::new(SearchCache {
                ready: std::collections::HashMap::new(),
                order: std::collections::VecDeque::new(),
                inflight: std::collections::HashSet::new(),
                most_recent: Vec::new(),
            })
        });
    let key: Key = (
        query.to_string(),
        root.to_string(),
        use_regex,
        whole_word,
        case_insensitive,
    );
    let mut cache = CACHE.lock();
    if let Some(v) = cache.ready.get(&key) {
        return v.clone();
    }
    // First request for this query: kick a background grep. The closure
    // publishes its result only if the query is still wanted, and the
    // cache is bounded so long sessions of typing cannot grow it.
    if cache.inflight.insert(key.clone()) {
        let job_key = key.clone();
        std::thread::spawn(move || {
            let val = project_search_blocking(
                &job_key.0, &job_key.1, job_key.2, job_key.3, job_key.4,
            );
            let mut cache = CACHE.lock();
            cache.inflight.remove(&job_key);
            cache.most_recent = val.clone();
            cache.ready.insert(job_key.clone(), val);
            cache.order.push_back(job_key);
            while cache.order.len() > 64 {
                if let Some(old) = cache.order.pop_front() {
                    cache.ready.remove(&old);
                }
            }
        });
    }
    cache.most_recent.clone()
}

/// Execute project-wide find-and-replace using sed. Returns the number of
/// files modified. In literal mode the search is escaped so every character
/// matches exactly, and case is folded only when `case_insensitive` is set,
/// so a replace matches exactly the text the user typed in the case they
/// typed it.
pub(crate) fn execute_project_replace(
    root: &str,
    search: &str,
    replace: &str,
    use_regex: bool,
    case_insensitive: bool,
) -> usize {
    if search.is_empty() {
        return 0;
    }
    // Find matching files first. `-F` keeps grep's selection literal in
    // non-regex mode so it agrees with the literal sed expression below.
    let mut grep_args: Vec<&str> = vec!["-rl"];
    if case_insensitive {
        grep_args.push("-i");
    }
    if !use_regex {
        grep_args.push("-F");
    }
    for inc in [
        "--include=*.rs",
        "--include=*.toml",
        "--include=*.json",
        "--include=*.md",
        "--include=*.txt",
        "--include=*.js",
        "--include=*.ts",
        "--include=*.py",
        "--include=*.go",
        "--include=*.c",
        "--include=*.h",
        "--include=*.cpp",
        "--include=*.java",
    ] {
        grep_args.push(inc);
    }
    grep_args.push(search);
    grep_args.push(root);
    let grep_out = std::process::Command::new("grep").args(&grep_args).output();
    let Ok(out) = grep_out else { return 0 };
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let files: Vec<&str> = stdout.lines().collect();
    if files.is_empty() {
        return 0;
    }
    // sed matches with BRE. In literal mode escape the BRE metacharacters
    // and the `/` delimiter so the search text is matched verbatim; in regex
    // mode only the delimiter is escaped so the user's pattern is preserved.
    let sed_search = if use_regex {
        search.replace('/', "\\/")
    } else {
        let mut escaped = String::with_capacity(search.len() + 8);
        for c in search.chars() {
            if matches!(c, '\\' | '.' | '*' | '[' | ']' | '^' | '$' | '/') {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        escaped
    };
    // In the replacement text only `\`, `&`, and the delimiter are special.
    let sed_replace = replace
        .replace('\\', "\\\\")
        .replace('/', "\\/")
        .replace('&', "\\&")
        .replace('\n', "\\n");
    let flags = if case_insensitive { "gi" } else { "g" };
    let sed_expr = format!("s/{sed_search}/{sed_replace}/{flags}");
    let mut count = 0usize;
    for file in &files {
        let file = file.trim();
        if file.is_empty() {
            continue;
        }
        let ok = if cfg!(target_os = "macos") {
            std::process::Command::new("sed")
                .args(["-i", "", "-e", &sed_expr, file])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("sed")
                .args(["-i", "-e", &sed_expr, file])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if ok {
            count += 1;
        }
    }
    count
}
