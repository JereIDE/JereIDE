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
    anchored: bool,
    kind: CompiledPatternKind,
}

#[derive(Clone)]
enum CompiledPatternKind {
    Line(Regex),
    Block {
        start_re: Regex,
        end_re: Regex,
        escape: Option<char>,
    },
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
        RawPattern::Line { type_, pattern } => Regex::new(pattern).ok().map(|re| CompiledPattern {
            type_: type_.clone(),
            anchored: pattern.starts_with('^'),
            kind: CompiledPatternKind::Line(re),
        }),
        RawPattern::Block {
            type_,
            start,
            end,
            escape,
        } => {
            let start_re = Regex::new(start).ok()?;
            let end_re = Regex::new(end).ok()?;
            let esc = escape.as_ref().and_then(|s| s.chars().next());
            Some(CompiledPattern {
                type_: type_.clone(),
                anchored: start.starts_with('^'),
                kind: CompiledPatternKind::Block {
                    start_re,
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

/// Applies a pattern that is known to start at `pos`. Returns the token to
/// emit and the next position to continue from.
fn apply_match(
    pattern: &CompiledPattern,
    text: &str,
    pattern_idx: usize,
    pos: usize,
    len: usize,
    symbols: &HashMap<String, String>,
    state: &mut HlState,
) -> Option<(Token, usize)> {
    match &pattern.kind {
        CompiledPatternKind::Line(re) => {
            let m = re.find(&text[pos..])?;
            let end = pos + m.end();
            let type_ = resolve_type(&pattern.type_, &text[pos..end], symbols);
            Some(((pos, end, type_), end))
        }
        CompiledPatternKind::Block {
            start_re, end_re, ..
        } => {
            let m = start_re.find(&text[pos..])?;
            let m_end = pos + m.end();
            let rest = &text[m_end..];
            if let Some(end_m) = end_re.find(rest) {
                let end = m_end + end_m.end();
                Some(((pos, end, pattern.type_.clone()), end))
            } else {
                *state = HlState::InBlock {
                    pattern_idx,
                    escaped: false,
                };
                Some(((pos, len, pattern.type_.clone()), len))
            }
        }
    }
}

fn tokenize(text: &str, def: &CompiledSyntax, state: &mut HlState) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let len = text.len();
    let mut pos = 0;

    // Precompute the matches of every non-anchored pattern over the whole text so
    // they can be looked up in O(1) instead of re-scanning the remaining text at
    // every position. Anchored (`^`) patterns are checked cheaply on the fly.
    let mut scans: Vec<(Vec<(usize, usize)>, usize)> = def
        .patterns
        .iter()
        .map(|p| {
            let matches = if p.anchored {
                Vec::new()
            } else {
                match &p.kind {
                    CompiledPatternKind::Line(re) => {
                        re.find_iter(text).map(|m| (m.start(), m.end())).collect()
                    }
                    CompiledPatternKind::Block { start_re, .. } => start_re
                        .find_iter(text)
                        .map(|m| (m.start(), m.end()))
                        .collect(),
                }
            };
            (matches, 0)
        })
        .collect();

    let advance = |scans: &mut Vec<(Vec<(usize, usize)>, usize)>, idx: usize, to: usize| {
        while scans[idx].1 < scans[idx].0.len() && scans[idx].0[scans[idx].1].0 < to {
            scans[idx].1 += 1;
        }
    };

    while pos < len {
        match state {
            HlState::Normal => {
                let mut matched = false;

                for (idx, pattern) in def.patterns.iter().enumerate() {
                    if pattern.anchored {
                        let m = match &pattern.kind {
                            CompiledPatternKind::Line(re) => re.find(&text[pos..]),
                            CompiledPatternKind::Block { start_re, .. } => {
                                start_re.find(&text[pos..])
                            }
                        };
                        if let Some(m) = m {
                            if m.start() == 0 {
                                if let Some((token, next)) =
                                    apply_match(pattern, text, idx, pos, len, &def.symbols, state)
                                {
                                    tokens.push(token);
                                    pos = next;
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    } else {
                        advance(&mut scans, idx, pos);
                        if let Some(&(m_start, _)) = scans[idx].0.get(scans[idx].1) {
                            if m_start == pos {
                                if let Some((token, next)) =
                                    apply_match(pattern, text, idx, pos, len, &def.symbols, state)
                                {
                                    tokens.push(token);
                                    pos = next;
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !matched {
                    let start = pos;
                    let mut next = len;

                    for (idx, pattern) in def.patterns.iter().enumerate() {
                        if pattern.anchored {
                            continue;
                        }
                        advance(&mut scans, idx, pos);
                        if let Some(&(m_start, _)) = scans[idx].0.get(scans[idx].1) {
                            if m_start < next {
                                next = m_start;
                            }
                        }
                    }

                    if def.patterns.iter().any(|p| p.anchored) {
                        let mut p = start;
                        while p < next {
                            let mut hit = false;
                            for pattern in &def.patterns {
                                if !pattern.anchored {
                                    continue;
                                }
                                let m = match &pattern.kind {
                                    CompiledPatternKind::Line(re) => re.find(&text[p..]),
                                    CompiledPatternKind::Block { start_re, .. } => {
                                        start_re.find(&text[p..])
                                    }
                                };
                                if let Some(m) = m {
                                    if m.start() == 0 {
                                        hit = true;
                                        break;
                                    }
                                }
                            }
                            if hit {
                                next = p;
                                break;
                            }
                            p = next_char_boundary(text, p);
                        }
                    }

                    if next <= start {
                        next = next_char_boundary(text, start);
                    }
                    tokens.push((start, next, "plain".to_string()));
                    pos = next;
                }
            }
            HlState::InBlock {
                pattern_idx,
                escaped,
            } => {
                let pattern = &def.patterns[*pattern_idx];
                if let CompiledPatternKind::Block { end_re, escape, .. } = &pattern.kind {
                    let rest = &text[pos..];
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
                                let end = pos + search_pos + end_m.end();
                                tokens.push((pos, end, pattern.type_.clone()));
                                pos = end;
                                *state = HlState::Normal;
                                found = true;
                                break;
                            }
                        }

                        search_pos = next_char_boundary(rest, search_pos);
                    }

                    if !found {
                        tokens.push((pos, len, pattern.type_.clone()));
                        pos = len;
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

fn tokens_to_job(text: &str, tokens: &[Token], font_id: &FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob {
        text: text.to_string(),
        wrap: egui::text::TextWrapping {
            max_width: f32::INFINITY,
            ..Default::default()
        },
        ..Default::default()
    };

    if tokens.is_empty() {
        if !text.is_empty() {
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: TextFormat::simple(font_id.clone(), TEXT_DEFAULT),
            });
        }
        return job;
    }

    for (start, end, type_) in tokens {
        let color = type_to_color(type_);
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: *start..*end,
            format: TextFormat::simple(font_id.clone(), color),
        });
    }

    job
}

pub struct SyntaxHighlighter {
    font_id: FontId,
    syntax_def: Option<CompiledSyntax>,
    cached_text: String,
    cached_job: egui::text::LayoutJob,
    state: HlState,
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
            state: HlState::Normal,
        }
    }

    pub fn highlight(&mut self, text: &str) -> &egui::text::LayoutJob {
        if text == self.cached_text && !self.cached_text.is_empty() {
            return &self.cached_job;
        }

        self.cached_text = text.to_string();

        let job = if let Some(ref def) = self.syntax_def {
            if text.is_empty() {
                let mut j = egui::text::LayoutJob::default();
                j.text = String::new();
                j
            } else {
                self.state = HlState::Normal;
                let tokens = tokenize(text, def, &mut self.state);
                tokens_to_job(text, &tokens, &self.font_id)
            }
        } else {
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
            job
        };

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
}
