use std::collections::HashMap;

/// A frequency-sorted word index built from document lines for instant
/// document-word autocomplete (no LSP dependency).
pub struct WordIndex {
    words: Vec<(String, usize)>,
    pub dirty: bool,
    /// Per-buffer change_id watermark so we only rebuild when the buffer
    /// actually changed.
    pub last_seen_change_id: HashMap<u64, i64>,
}

impl WordIndex {
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            dirty: false,
            last_seen_change_id: HashMap::new(),
        }
    }

    /// Rebuild the word index from the given lines.
    pub fn rebuild(&mut self, lines: &[String]) {
        let mut freq: HashMap<String, usize> = HashMap::new();
        for line in lines {
            let chars: Vec<char> = line.chars().collect();
            let len = chars.len();
            let mut i = 0;
            while i < len {
                if !chars[i].is_alphanumeric() && chars[i] != '_' {
                    i += 1;
                    continue;
                }
                let start = i;
                i += 1;
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                if i - start >= 2 {
                    let word: String = chars[start..i].iter().collect();
                    *freq.entry(word).or_insert(0) += 1;
                }
            }
        }
        let mut words: Vec<(String, usize)> = freq.into_iter().collect();
        words.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        self.words = words;
        self.dirty = false;
    }

    /// Query words matching `prefix`. Returns up to `max` results in the
    /// same (label, detail, insert_text) format the completion popup expects.
    pub fn query(&self, prefix: &str, max: usize) -> Vec<(String, String, String)> {
        if prefix.is_empty() {
            return Vec::new();
        }
        self.words
            .iter()
            .filter(|(w, _)| w.len() > prefix.len() && w.starts_with(prefix))
            .take(max)
            .map(|(w, _)| (w.clone(), String::new(), w.clone()))
            .collect()
    }
}
