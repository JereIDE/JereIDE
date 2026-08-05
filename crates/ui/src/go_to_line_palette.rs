use eframe::egui;
use jereide_widgets::singleline_palette::SinglelinePalette;

pub struct GoToLinePalette {
    palette: SinglelinePalette,
}

impl GoToLinePalette {
    pub fn new() -> Self {
        Self {
            palette: SinglelinePalette::new(),
        }
    }

    /// Returns the 1-based target line to jump to, or `None` while the
    /// palette is still open.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        total_lines: usize,
        open: &mut bool,
    ) -> Option<usize> {
        let hint = format!("Enter Line... (1-{})", total_lines);
        self.palette
            .render(ctx, "Go to Line", &hint, open)
            .and_then(|input| input.trim().parse().ok())
            .map(|line: usize| line.clamp(1, total_lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_palette_returns_none() {
        let mut palette = GoToLinePalette::new();
        let ctx = egui::Context::default();
        let mut open = false;
        assert_eq!(palette.render(&ctx, 100, &mut open), None);
        assert!(!open);
    }
}
