use std::collections::HashMap;

use eframe::egui::{self, Color32, FontId, TextFormat};
use regex::Regex;
use serde::Deserialize;

use jereide_settings::{
    SYNTAX_COMMENT, SYNTAX_FUNCTION, SYNTAX_KEYWORD, SYNTAX_KEYWORD2, SYNTAX_LITERAL,
    SYNTAX_NUMBER, SYNTAX_OPERATOR, SYNTAX_STRING, TEXT_DEFAULT,
};

// ---------------------------------------------------------------------------
// Flat JSON schema — no $ref, no Lua patterns, just regex
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SyntaxFile {
    syntax: SyntaxDef,
}

#[derive(Debug, Deserialize)]
struct SyntaxDef {
    name: String,
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

// ---------------------------------------------------------------------------
// Compiled syntax definition (regexes compiled at load time)
// ---------------------------------------------------------------------------

struct CompiledPattern {
    type_: String,
    kind: CompiledPatternKind,
}

enum CompiledPatternKind {
    Line(Regex),
    Block {
        start_re: Regex,
        end_re: Regex,
        escape: Option<char>,
    },
}

struct CompiledSyntax {
    _name: String,
    _file_patterns: Vec<Regex>,
    symbols: HashMap<String, String>,
    patterns: Vec<CompiledPattern>,
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_syntax(data_dir: &std::path::Path, extension: &str) -> Option<CompiledSyntax> {
    let path = data_dir.join(format!("{extension}.json"));
    let content = std::fs::read_to_string(&path).ok()?;
    let file: SyntaxFile = serde_json::from_str(&content).ok()?;

    let def = file.syntax;

    let _file_patterns: Vec<Regex> = def
        .files
        .iter()
        .filter_map(|s| Regex::new(s).ok())
        .collect();

    let patterns: Vec<CompiledPattern> = def
        .patterns
        .iter()
        .filter_map(|rp| compile_pattern(rp))
        .collect();

    Some(CompiledSyntax {
        _name: def.name,
        _file_patterns,
        symbols: def.symbols,
        patterns,
    })
}

fn compile_pattern(rp: &RawPattern) -> Option<CompiledPattern> {
    match rp {
        RawPattern::Line { type_, pattern } => Regex::new(pattern).ok().map(|re| CompiledPattern {
            type_: type_.clone(),
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
                kind: CompiledPatternKind::Block {
                    start_re,
                    end_re,
                    escape: esc,
                },
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum HlState {
    Normal,
    InBlock { pattern_idx: usize, escaped: bool },
}

type Token = (usize, usize, String);

fn tokenize(text: &str, def: &CompiledSyntax, state: &mut HlState) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        match state {
            HlState::Normal => {
                let mut matched = false;

                for (idx, pattern) in def.patterns.iter().enumerate() {
                    match &pattern.kind {
                        CompiledPatternKind::Line(re) => {
                            if let Some(m) = re.find(&text[pos..]) {
                                if m.start() == 0 {
                                    let end = pos + m.end();
                                    let type_ =
                                        resolve_type(&pattern.type_, &text[pos..end], &def.symbols);
                                    tokens.push((pos, end, type_));
                                    pos = end;
                                    matched = true;
                                    break;
                                }
                            }
                        }
                        CompiledPatternKind::Block {
                            start_re,
                            end_re,
                            escape: _,
                        } => {
                            if let Some(m) = start_re.find(&text[pos..]) {
                                if m.start() == 0 {
                                    let rest = &text[pos + m.end()..];
                                    if let Some(end_m) = end_re.find(rest) {
                                        let end = pos + m.end() + end_m.end();
                                        tokens.push((pos, end, pattern.type_.clone()));
                                        pos = end;
                                    } else {
                                        tokens.push((pos, len, pattern.type_.clone()));
                                        *state = HlState::InBlock {
                                            pattern_idx: idx,
                                            escaped: false,
                                        };
                                        pos = len;
                                    }
                                    matched = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !matched {
                    let start = pos;
                    pos += 1;
                    while pos < len {
                        let mut any_match = false;
                        for pattern in &def.patterns {
                            match &pattern.kind {
                                CompiledPatternKind::Line(re) => {
                                    if let Some(m) = re.find(&text[pos..]) {
                                        if m.start() == 0 {
                                            any_match = true;
                                            break;
                                        }
                                    }
                                }
                                CompiledPatternKind::Block { start_re, .. } => {
                                    if let Some(m) = start_re.find(&text[pos..]) {
                                        if m.start() == 0 {
                                            any_match = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if any_match {
                            break;
                        }
                        pos += 1;
                    }
                    tokens.push((start, pos, "plain".to_string()));
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
                                search_pos += 1;
                                continue;
                            }
                            if *escaped {
                                *escaped = false;
                                search_pos += 1;
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

                        search_pos += 1;
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

// ---------------------------------------------------------------------------
// Token to LayoutJob conversion
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Data directory discovery
// ---------------------------------------------------------------------------

fn find_data_dir() -> Option<std::path::PathBuf> {
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct SyntaxHighlighter {
    font_id: FontId,
    syntax_def: Option<CompiledSyntax>,
    cached_text: String,
    cached_job: egui::text::LayoutJob,
    state: HlState,
}

impl SyntaxHighlighter {
    pub fn new(font_size: f32, extension: Option<&str>) -> Self {
        let font_id = FontId::monospace(font_size);
        let syntax_def = extension.and_then(|ext| {
            let data_dir = find_data_dir()?;
            load_syntax(&data_dir, ext)
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
