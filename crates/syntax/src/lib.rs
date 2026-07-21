use eframe::egui::{self, Color32, FontId, TextFormat};
use jereide_settings::TEXT_DEFAULT;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{HighlightState, Theme, ThemeSet};
use syntect::parsing::{ParseState, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Get a default syntax set or something.
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Default theme sets?
fn theme_set() -> &'static ThemeSet {
    static TS: OnceLock<ThemeSet> = OnceLock::new();
    TS.get_or_init(ThemeSet::load_defaults)
}

/// For incremental highlighting
#[derive(Clone)]
struct CachedLine {
    content: String,
    sections: Vec<(usize, usize, Color32)>,
    hl_state: HighlightState,
    parse_state: ParseState,
}

/// Max lines to re-highlight with syntect per frame. Beyond this,
/// lines use default coloring; the next frame resumes from where it left off.
const MAX_SYNTECT_LINES: usize = 200;

/// Includes lots of metadata
pub struct SyntaxHighlighter {
    font_id: FontId,
    syntax: &'static SyntaxReference,
    theme: &'static Theme,
    lines: Vec<CachedLine>,
    cached_text: String,
    cached_job: Option<egui::text::LayoutJob>,
    resume_from: Option<usize>,
    pending_update: bool,
}

/// Syntax highlighter with syntect
impl SyntaxHighlighter {
    pub fn new(font_size: f32, extension: Option<&str>) -> Self {
        let ss = syntax_set();
        // Use the extension(main.rs -> Rust)
        // or fall back to plain text
        let syntax = extension
            .and_then(|ext| ss.find_syntax_by_extension(ext))
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let ts = theme_set();

        // InspiredGitHub is the best in all the
        // defaults, falls back to base16-ocean
        let theme = ts
            .themes
            .get("InspiredGitHub")
            .or_else(|| ts.themes.get("base16-ocean.light"))
            .unwrap_or_else(|| {
                ts.themes
                    .values()
                    .next()
                    .expect("at least one default theme")
            });

        // Monospace font, of course.
        Self {
            font_id: FontId::monospace(font_size),
            syntax,
            theme,
            lines: Vec::new(),
            cached_text: String::new(),
            cached_job: None,
            resume_from: None,
            pending_update: false,
        }
    }

    /// Build a plain (unhighlighted) LayoutJob — fast, no syntect.
    fn build_plain_job(&self, text: &str) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob {
            text: text.to_string(),
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
    }

    /// Highlights.
    ///
    /// When text changes, returns a plain layout immediately (deferred).
    /// On the next frame (text unchanged), performs full syntect highlighting
    /// and caches the result.
    pub fn highlight(&mut self, text: &str) -> egui::text::LayoutJob {
        if text.is_empty() {
            self.lines.clear();
            self.cached_text.clear();
            self.cached_job = None;
            self.resume_from = None;
            self.pending_update = false;
            return egui::text::LayoutJob::default();
        }

        // --- Text changed: defer highlighting, return plain layout ---
        if text != self.cached_text {
            self.cached_text = text.to_string();
            self.resume_from = None;
            self.pending_update = true;
            self.cached_job = None;
            return self.build_plain_job(text);
        }

        // --- Text unchanged, no pending work: return cached job ---
        if !self.pending_update && self.resume_from.is_none() {
            if let Some(job) = &self.cached_job {
                return job.clone();
            }
            // No cached job yet (shouldn't happen, but be safe)
            return self.build_plain_job(text);
        }

        // --- Text unchanged, but pending_update or resume_from is set ---
        // Perform the deferred highlighting now.
        self.pending_update = false;

        let ss = syntax_set();
        let new_lines: Vec<&str> = LinesWithEndings::from(text).collect();

        let first_diff = match self.resume_from {
            Some(r) => r.min(new_lines.len()),
            None => self
                .lines
                .iter()
                .zip(new_lines.iter())
                .position(|(cached, &new)| cached.content != new)
                .unwrap_or(usize::MAX)
                .min(self.lines.len())
                .min(new_lines.len()),
        };

        // All lines match — just rebuild the job from current lines
        if first_diff == self.lines.len() && self.lines.len() == new_lines.len() {
            let job = self.build_job();
            self.cached_job = Some(job.clone());
            return job;
        }

        let mut old_remainder: Vec<CachedLine> = self.lines.drain(first_diff..).collect();

        let mut hl = if first_diff == 0 {
            HighlightLines::new(self.syntax, self.theme)
        } else {
            let prev = &self.lines[first_diff - 1];
            HighlightLines::from_state(self.theme, prev.hl_state.clone(), prev.parse_state.clone())
        };

        let total_new = new_lines.len() - first_diff;
        let to_highlight = total_new.min(MAX_SYNTECT_LINES);
        let mut new_cache: Vec<CachedLine> = Vec::with_capacity(to_highlight);

        for (rel_idx, &line) in new_lines[first_diff..first_diff + to_highlight]
            .iter()
            .enumerate()
        {
            let result = hl.highlight_line(line, ss);
            let (hls, ps) = hl.state();

            let sections = if let Ok(ref ranges) = result {
                ranges
                    .iter()
                    .map(|(style, part)| {
                        let color = Color32::from_rgba_unmultiplied(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            style.foreground.a,
                        );
                        let part_start = part.as_ptr() as usize - line.as_ptr() as usize;
                        (part_start, part_start + part.len(), color)
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let should_stop = if rel_idx < old_remainder.len()
                && hls == old_remainder[rel_idx].hl_state
                && ps == old_remainder[rel_idx].parse_state
            {
                let remaining_new = &new_lines[first_diff + rel_idx + 1..];
                let remaining_old = &old_remainder[rel_idx + 1..];
                remaining_new.len() == remaining_old.len()
                    && remaining_new
                        .iter()
                        .zip(remaining_old.iter())
                        .all(|(&n, o)| n == o.content)
            } else {
                false
            };

            new_cache.push(CachedLine {
                content: line.to_string(),
                sections,
                hl_state: hls.clone(),
                parse_state: ps.clone(),
            });

            if should_stop {
                new_cache.extend(old_remainder.split_off(rel_idx + 1));
                break;
            }

            hl = HighlightLines::from_state(self.theme, hls, ps);
        }

        self.lines.extend(new_cache);

        // Track remaining unhighlighted lines for next frame
        if self.lines.len() < new_lines.len() {
            self.resume_from = Some(self.lines.len());
        } else {
            self.resume_from = None;
        }

        // If work remains, another frame is needed
        if self.resume_from.is_some() {
            self.pending_update = true;
        }

        let job = self.build_job();
        self.cached_job = Some(job.clone());
        job
    }

    fn build_job(&self) -> egui::text::LayoutJob {
        let text = &self.cached_text;
        let mut job = egui::text::LayoutJob {
            text: text.clone(),
            ..Default::default()
        };

        if self.lines.is_empty() {
            if !text.is_empty() {
                job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: 0..text.len(),
                    format: TextFormat::simple(self.font_id.clone(), TEXT_DEFAULT),
                });
            }
            return job;
        }

        let default_fmt = TextFormat::simple(self.font_id.clone(), TEXT_DEFAULT);
        let mut cursor = 0;
        let mut line_start = 0;

        for line in &self.lines {
            for &(start, end, color) in &line.sections {
                let abs_start = line_start + start;
                let abs_end = line_start + end;
                if abs_start > cursor {
                    job.sections.push(egui::text::LayoutSection {
                        leading_space: 0.0,
                        byte_range: cursor..abs_start,
                        format: default_fmt.clone(),
                    });
                }
                job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: abs_start..abs_end,
                    format: TextFormat::simple(self.font_id.clone(), color),
                });
                cursor = abs_end;
            }
            line_start += line.content.len();
        }
        if cursor < text.len() {
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: cursor..text.len(),
                format: default_fmt,
            });
        }

        job
    }
}
