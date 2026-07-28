use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct LanguagesFile {
    languages: Vec<LanguageDef>,
}

#[derive(Debug, Deserialize, Clone)]
struct LanguageDef {
    extensions: Vec<String>,
    name: String,
    indent_after: Vec<String>,
    file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub indent_triggers: Vec<char>,
    pub syntax_file: Option<String>,
}

pub fn data_dir() -> Option<std::path::PathBuf> {
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

fn load_languages() -> Vec<LanguageDef> {
    let dir = match data_dir() {
        Some(d) => d,
        None => return vec![],
    };
    let path = dir.join("languages.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let file: LanguagesFile = match serde_json::from_str(&content) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    file.languages
}

fn get_languages() -> &'static [LanguageDef] {
    static CACHE: OnceLock<Vec<LanguageDef>> = OnceLock::new();
    CACHE.get_or_init(load_languages)
}

pub fn lookup_language(extension: Option<&str>) -> Option<LanguageInfo> {
    let ext = extension.unwrap_or("");
    let def = get_languages()
        .iter()
        .find(|lang| lang.extensions.iter().any(|e| e == ext))?;
    Some(LanguageInfo {
        name: def.name.clone(),
        indent_triggers: def
            .indent_after
            .iter()
            .filter_map(|s| s.chars().next())
            .collect(),
        syntax_file: def.file.clone(),
    })
}

pub fn lookup_language_by_path(path: Option<&str>) -> Option<LanguageInfo> {
    let ext = path
        .and_then(|p| Path::new(p).extension())
        .and_then(|e| e.to_str());
    lookup_language(ext)
}
