use std::collections::HashMap;

use eframe::egui::{self, Color32, FontId, TextFormat};
use jereide_data::data_dir;
use regex::Regex;
use serde::Deserialize;

use jereide_settings::{
    SYNTAX_CODE, SYNTAX_COMMENT, SYNTAX_EMPHASIS, SYNTAX_FUNCTION, SYNTAX_HEADING, SYNTAX_KEYWORD,
    SYNTAX_KEYWORD2, SYNTAX_LINK, SYNTAX_LITERAL, SYNTAX_NUMBER, SYNTAX_OPERATOR, SYNTAX_STRING,
    TEXT_DEFAULT,
};

#[derive(Debug, Deserialize)]
struct SyntaxFile {
    syntax: SyntaxDef,
}

#[derive(Debug, Deserialize)]
struct SyntaxDef {
    name: String,
    #[allow(dead_code)]
    files: Vec<String>,
    symbols: HashMap<String, String>,
    patterns: Vec<RawPattern>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawPattern {
    Line {
        #[serde(rename = "type")]
        type_: String,
        pattern: String,
    },
    Block {
        #[serde(rename = "type")]
        type_: String,
        start: String,
        end: String,
        #[serde(default)]
        escape: Option<String>,
    },
}

#[derive(Clone)]
struct CompiledPattern {
    type_: String,
    /// Regex used to find candidate match positions (leading `^` stripped, since
    /// this tokenizer anchors to the current position anyway).
    search_re: Regex,
    kind: CompiledPatternKind,
}

#[derive(Clone)]
enum CompiledPatternKind {
    Line,
    Block { end_re: Regex, escape: Option<char> },
}

#[derive(Clone)]
struct CompiledSyntax {
    _name: String,
    symbols: HashMap<String, String>,
    patterns: Vec<CompiledPattern>,
}

fn load_syntax(data_dir: &std::path::Path, file: &str) -> Option<CompiledSyntax> {
    let path = data_dir.join(format!("{file}.json"));
    let content = std::fs::read_to_string(&path).ok()?;
    let file: SyntaxFile = serde_json::from_str(&content).ok()?;

    let def = file.syntax;

    let patterns: Vec<CompiledPattern> = def
        .patterns
        .iter()
        .filter_map(|rp| compile_pattern(rp))
        .collect();

    Some(CompiledSyntax {
        _name: def.name,
        symbols: def.symbols,
        patterns,
    })
}

fn compile_pattern(rp: &RawPattern) -> Option<CompiledPattern> {
    match rp {
        RawPattern::Line { type_, pattern } => {
            let search_re = Regex::new(pattern.strip_prefix('^').unwrap_or(pattern)).ok()?;
            Some(CompiledPattern {
                type_: type_.clone(),
                search_re,
                kind: CompiledPatternKind::Line,
            })
        }
        RawPattern::Block {
            type_,
            start,
            end,
            escape,
        } => {
            let search_re = Regex::new(start.strip_prefix('^').unwrap_or(start)).ok()?;
            let end_re = Regex::new(end).ok()?;
            let esc = escape.as_ref().and_then(|s| s.chars().next());
            Some(CompiledPattern {
                type_: type_.clone(),
                search_re,
                kind: CompiledPatternKind::Block {
                    end_re,
                    escape: esc,
                },
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum HlState {
    Normal,
    InBlock { pattern_idx: usize, escaped: bool },
}

type Token = (usize, usize, String);

fn next_char_boundary(text: &str, from: usize) -> usize {
    match text[from..].chars().next() {
        Some(c) => from + c.len_utf8(),
        None => from,
    }
}

/// Applies a pattern that is known to start at `m_start` (with a match ending
/// at `m_end`). Returns the token to emit and the next position to continue from.
fn apply_match(
    pattern: &CompiledPattern,
    text: &str,
    pattern_idx: usize,
    m_start: usize,
    m_end: usize,
    len: usize,
    symbols: &HashMap<String, String>,
    state: &mut HlState,
) -> Option<(Token, usize)> {
    match &pattern.kind {
        CompiledPatternKind::Line => {
            let type_ = resolve_type(&pattern.type_, &text[m_start..m_end], symbols);
            Some(((m_start, m_end, type_), m_end))
        }
        CompiledPatternKind::Block { end_re, .. } => {
            let rest = &text[m_end..len];
            if let Some(end_m) = end_re.find(rest) {
                let end = m_end + end_m.end();
                Some(((m_start, end, pattern.type_.clone()), end))
            } else {
                *state = HlState::InBlock {
                    pattern_idx,
                    escaped: false,
                };
                Some(((m_start, len, pattern.type_.clone()), len))
            }
        }
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts.push(text.len());
    starts
}

fn first_diff(a: &str, b: &str) -> usize {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    let mut i = 0;
    while i < n && ab[i] == bb[i] {
        i += 1;
    }
    i
}

fn common_suffix_len(a: &str, b: &str) -> usize {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    let mut i = 0;
    while i < n && ab[ab.len() - 1 - i] == bb[bb.len() - 1 - i] {
        i += 1;
    }
    i
}

fn tokenize_range(
    text: &str,
    start: usize,
    end: usize,
    def: &CompiledSyntax,
    state: &mut HlState,
) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut pos = start;
    let slice = &text[start..end];

    // Precompute the matches of every pattern over the tokenized range so they
    // can be looked up in O(1) instead of re-scanning at every position.
    // Anchored (`^`) patterns use a `^`-stripped search regex, which matches the
    // tokenizer's "anchor at current position" behavior.
    let mut scans: Vec<(Vec<(usize, usize)>, usize)> = def
        .patterns
        .iter()
        .map(|p| {
            let matches = p
                .search_re
                .find_iter(slice)
                .map(|m| (start + m.start(), start + m.end()))
                .collect();
            (matches, 0)
        })
        .collect();

    let advance = |scans: &mut Vec<(Vec<(usize, usize)>, usize)>, idx: usize, to: usize| {
        while scans[idx].1 < scans[idx].0.len() && scans[idx].0[scans[idx].1].0 < to {
            scans[idx].1 += 1;
        }
    };

    while pos < end {
        match state {
            HlState::Normal => {
                let mut matched = false;

                for (idx, pattern) in def.patterns.iter().enumerate() {
                    advance(&mut scans, idx, pos);
                    if let Some(&(m_start, m_end)) = scans[idx].0.get(scans[idx].1) {
                        if m_start == pos {
                            if let Some((token, next)) = apply_match(
                                pattern,
                                text,
                                idx,
                                m_start,
                                m_end,
                                end,
                                &def.symbols,
                                state,
                            ) {
                                tokens.push(token);
                                pos = next;
                                matched = true;
                                break;
                            }
                        }
                    }
                }

                if !matched {
                    let s = pos;
                    let mut next = end;

                    for (idx, _) in def.patterns.iter().enumerate() {
                        advance(&mut scans, idx, pos);
                        if let Some(&(m_start, _)) = scans[idx].0.get(scans[idx].1) {
                            if m_start < next {
                                next = m_start;
                            }
                        }
                    }

                    if next <= s {
                        next = next_char_boundary(text, s);
                    }
                    tokens.push((s, next, "plain".to_string()));
                    pos = next;
                }
            }
            HlState::InBlock {
                pattern_idx,
                escaped,
            } => {
                let pattern = &def.patterns[*pattern_idx];
                if let CompiledPatternKind::Block { end_re, escape, .. } = &pattern.kind {
                    let rest = &text[pos..end];
                    let mut search_pos = 0;
                    let mut found = false;
                    let bytes_rest = rest.as_bytes();

                    while search_pos < bytes_rest.len() {
                        if let Some(esc) = escape {
                            if !*escaped && bytes_rest[search_pos] == *esc as u8 {
                                *escaped = true;
                                search_pos = next_char_boundary(rest, search_pos);
                                continue;
                            }
                            if *escaped {
                                *escaped = false;
                                search_pos = next_char_boundary(rest, search_pos);
                                continue;
                            }
                        }

                        if let Some(end_m) = end_re.find(&rest[search_pos..]) {
                            if end_m.start() == 0 {
                                let e = pos + search_pos + end_m.end();
                                tokens.push((pos, e, pattern.type_.clone()));
                                pos = e;
                                *state = HlState::Normal;
                                found = true;
                                break;
                            }
                        }

                        search_pos = next_char_boundary(rest, search_pos);
                    }

                    if !found {
                        tokens.push((pos, end, pattern.type_.clone()));
                        pos = end;
                    }
                } else {
                    *state = HlState::Normal;
                }
            }
        }
    }

    tokens
}

fn resolve_type(
    pattern_type: &str,
    matched_text: &str,
    symbols: &HashMap<String, String>,
) -> String {
    if pattern_type == "symbol" || pattern_type == "function" {
        if let Some(sym_type) = symbols.get(matched_text) {
            return sym_type.clone();
        }
    }
    pattern_type.to_string()
}

fn type_to_color(type_: &str) -> Color32 {
    match type_ {
        "keyword" => SYNTAX_KEYWORD,
        "keyword2" => SYNTAX_KEYWORD2,
        "string" => SYNTAX_STRING,
        "comment" => SYNTAX_COMMENT,
        "number" => SYNTAX_NUMBER,
        "operator" => SYNTAX_OPERATOR,
        "function" => SYNTAX_FUNCTION,
        "literal" => SYNTAX_LITERAL,
        "heading" => SYNTAX_HEADING,
        "code" => SYNTAX_CODE,
        "emphasis" => SYNTAX_EMPHASIS,
        "link" => SYNTAX_LINK,
        _ => TEXT_DEFAULT,
    }
}

pub struct SyntaxHighlighter {
    font_id: FontId,
    syntax_def: Option<CompiledSyntax>,
    cached_text: String,
    cached_job: egui::text::LayoutJob,
    line_starts: Vec<usize>,
    line_states: Vec<HlState>,
    line_tokens: Vec<Vec<(usize, usize, String)>>,
}

impl SyntaxHighlighter {
    pub fn new(font_size: f32, syntax_file: Option<&str>) -> Self {
        let font_id = FontId::monospace(font_size);
        let syntax_def = syntax_file.and_then(|file| {
            let dir = data_dir()?;
            load_syntax(&dir, file)
        });

        Self {
            font_id,
            syntax_def,
            cached_text: String::new(),
            cached_job: egui::text::LayoutJob::default(),
            line_starts: vec![0],
            line_states: vec![HlState::Normal],
            line_tokens: vec![vec![]],
        }
    }

    pub fn highlight(&mut self, text: &str) -> &egui::text::LayoutJob {
        if text == self.cached_text && !self.cached_text.is_empty() {
            return &self.cached_job;
        }

        let Some(def) = &self.syntax_def else {
            let mut job = egui::text::LayoutJob {
                text: text.to_string(),
                wrap: egui::text::TextWrapping {
                    max_width: f32::INFINITY,
                    ..Default::default()
                },
                ..Default::default()
            };
            if !text.is_empty() {
                job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: 0..text.len(),
                    format: TextFormat::simple(self.font_id.clone(), TEXT_DEFAULT),
                });
            }
            self.cached_text = text.to_string();
            self.cached_job = job;
            return &self.cached_job;
        };

        if text.is_empty() {
            self.cached_text = String::new();
            self.line_starts = vec![0];
            self.line_states = vec![HlState::Normal];
            self.line_tokens = vec![vec![]];
            let mut j = egui::text::LayoutJob::default();
            j.text = String::new();
            self.cached_job = j;
            return &self.cached_job;
        }

        let new_starts = line_starts(text);
        let new_lines = new_starts.len() - 1;

        // First line whose content changed; everything before it is reusable.
        let first_changed = if self.cached_text.is_empty() {
            0
        } else {
            let diff = first_diff(text, &self.cached_text);
            let mut l = new_starts.iter().filter(|&&s| s <= diff).count();
            l = l.saturating_sub(1);
            l.min(new_lines.saturating_sub(1))
        };

        let mut new_line_tokens: Vec<Vec<(usize, usize, String)>> = Vec::with_capacity(new_lines);
        let mut new_line_states: Vec<HlState> = Vec::with_capacity(new_lines + 1);

        for i in 0..first_changed {
            let old_line_start = self.line_starts[i];
            let new_line_start = new_starts[i];
            new_line_tokens.push(
                self.line_tokens
                    .get(i)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(s, e, t)| {
                        (
                            new_line_start + (s - old_line_start),
                            new_line_start + (e - old_line_start),
                            t,
                        )
                    })
                    .collect(),
            );
            new_line_states.push(self.line_states.get(i).cloned().unwrap_or(HlState::Normal));
        }

        let mut state = if first_changed == 0 {
            HlState::Normal
        } else {
            self.line_states
                .get(first_changed)
                .cloned()
                .unwrap_or(HlState::Normal)
        };
        new_line_states.push(state.clone());

        let mut stop_at = new_lines;
        let same_line_count = self.line_starts.len() == new_starts.len();
        let suffix_start = text.len() - common_suffix_len(text, &self.cached_text);
        for i in first_changed..new_lines {
            let tokens = tokenize_range(text, new_starts[i], new_starts[i + 1], def, &mut state);
            new_line_tokens.push(tokens);
            new_line_states.push(state.clone());

            let boundary = i + 1;
            if same_line_count
                && self.line_states.get(boundary) == Some(&state)
                && new_starts[boundary] >= suffix_start
            {
                stop_at = boundary;
                break;
            }
        }

        for i in stop_at..new_lines {
            let old_line_start = self.line_starts[i];
            let new_line_start = new_starts[i];
            new_line_tokens.push(
                self.line_tokens
                    .get(i)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(s, e, t)| {
                        (
                            new_line_start + (s - old_line_start),
                            new_line_start + (e - old_line_start),
                            t,
                        )
                    })
                    .collect(),
            );
            new_line_states.push(
                self.line_states
                    .get(i + 1)
                    .cloned()
                    .unwrap_or(HlState::Normal),
            );
        }

        let mut job = egui::text::LayoutJob {
            text: text.to_string(),
            wrap: egui::text::TextWrapping {
                max_width: f32::INFINITY,
                ..Default::default()
            },
            ..Default::default()
        };
        for line in &new_line_tokens {
            for &(s, e, ref t) in line {
                let color = type_to_color(t);
                job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: s..e,
                    format: TextFormat::simple(self.font_id.clone(), color),
                });
            }
        }
        if job.sections.is_empty() && !text.is_empty() {
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: TextFormat::simple(self.font_id.clone(), TEXT_DEFAULT),
            });
        }

        self.cached_text = text.to_string();
        self.line_starts = new_starts;
        self.line_states = new_line_states;
        self.line_tokens = new_line_tokens;
        self.cached_job = job;
        &self.cached_job
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section_colors(text: &str) -> Vec<(usize, usize, Color32)> {
        let mut hl = SyntaxHighlighter::new(14.0, Some("md"));
        let job = hl.highlight(text);
        job.sections
            .iter()
            .map(|s| (s.byte_range.start, s.byte_range.end, s.format.color))
            .collect()
    }

    #[test]
    fn markdown_loads_and_highlights_heading() {
        let colors = section_colors("# Hello\n");
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_HEADING),
            "expected a heading-colored section, got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_highlights_inline_code() {
        let colors = section_colors("Use `x` here\n");
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_CODE),
            "expected a code-colored section, got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_highlights_bold() {
        let colors = section_colors("some **bold** text\n");
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_EMPHASIS),
            "expected an emphasis-colored section, got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_highlights_fenced_code_block() {
        let text = "```rust\nlet x = 1;\n```\n";
        let colors = section_colors(text);
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_CODE),
            "expected a code-colored section for the fenced block, got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_fenced_block_terminates_at_closing_fence() {
        let text = "```rust\nlet x = 1;\n```\n# After\n";
        let colors = section_colors(text);
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_HEADING),
            "expected content after the closing fence to be highlighted (not swallowed by the \
             code block), got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_multibyte_utf8_does_not_panic() {
        let text = "Hello × world\n```rust\nlet s = \"×\";\n```\n# × Heading\n";
        let colors = section_colors(text);
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_HEADING),
            "expected a heading-colored section, got {:?}",
            colors
        );
    }

    #[test]
    fn markdown_large_file_highlights_quickly() {
        let mut text = String::new();
        for i in 0..2000 {
            text.push_str(&format!(
                "# Heading {i}\nSome prose *emphasized* and **bold** here.\n\n"
            ));
        }
        let colors = section_colors(&text);
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_HEADING),
            "expected headings to be highlighted in a large file"
        );
        assert!(
            colors.iter().any(|&(_, _, c)| c == SYNTAX_EMPHASIS),
            "expected emphasis to be highlighted in a large file"
        );
    }

    fn assert_same(hl: &mut SyntaxHighlighter, text: &str) {
        let job = hl.highlight(text).clone();
        let mut fresh = SyntaxHighlighter::new(14.0, Some("md"));
        let full = fresh.highlight(text);
        assert_eq!(
            job.sections.len(),
            full.sections.len(),
            "section count mismatch for {:?}",
            text
        );
        for (a, b) in job.sections.iter().zip(full.sections.iter()) {
            assert_eq!(a.byte_range, b.byte_range, "range mismatch for {:?}", text);
            assert_eq!(
                a.format.color, b.format.color,
                "color mismatch for {:?}",
                text
            );
        }
    }

    #[test]
    fn incremental_typing_matches_full_highlight() {
        let mut hl = SyntaxHighlighter::new(14.0, Some("md"));
        let base = "# H1\nSome *it* text\n```rust\nlet x = 1;\n```\n# H2\nlast line\n";
        hl.highlight(base);

        // Insert a bold word in the middle of line 1.
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** text\n```rust\nlet x = 1;\n```\n# H2\nlast line\n",
        );
        // Edit inside a code block.
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** text\n```rust\nlet x = 2;\n```\n# H2\nlast line\n",
        );
        // Insert a new line.
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** text\n\n```rust\nlet x = 2;\n```\n# H2\nlast line\n",
        );
        // Delete a line.
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** text\n```rust\nlet x = 2;\n```\nlast line\n",
        );
        // Edit the last line.
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** text\n```rust\nlet x = 2;\n```\n# H2\nlast **bold**\n",
        );
        // Delete a character from the middle line (same line count; tests negative shift).
        assert_same(
            &mut hl,
            "# H1\nSome *it* **b** tex\n```rust\nlet x = 2;\n```\n# H2\nlast **bold**\n",
        );
        // Insert a character near the start of the first line.
        assert_same(
            &mut hl,
            "# H1X\nSome *it* **b** tex\n```rust\nlet x = 2;\n```\n# H2\nlast **bold**\n",
        );
    }
}
