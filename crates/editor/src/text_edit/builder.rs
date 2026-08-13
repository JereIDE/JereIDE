use std::sync::Arc;

use eframe::egui;
use egui::{
    Align, Align2, AsIdSalt, AtomExt as _, AtomKind, AtomLayout, Atoms, Color32, Context,
    CursorIcon, Event, EventFilter, FontSelection, Frame, Id, IdSalt, ImeEvent, IntoAtoms,
    IntoSizedResult, Key, KeyboardShortcut, Margin, Modifiers, NumExt as _, Rect, Response, Sense,
    SizedAtomKind, TextStyle, TextWrapMode, Ui, Vec2, Widget, WidgetInfo, WidgetWithState,
    emath::TSTransform,
    epaint,
    os::OperatingSystem,
    output::OutputEvent,
    pos2, response,
    text::{CCursor, Galley, LayoutJob},
    text_selection,
    text_selection::{CCursorRange, text_cursor_state::cursor_rect, visuals::paint_text_selection},
    vec2,
};

use eframe::egui::accesskit;

use super::state::TextEditCursorPurpose;
use super::{TextBuffer, TextEditOutput, TextEditState};

#[derive(Clone)]
pub struct GutterConfig {
    pub padding_left: f32,
    pub padding_right: f32,
    pub digit_width: f32,
    pub line_number_right_offset: f32,
    pub current_line_color: egui::Color32,
    pub muted_color: egui::Color32,
    pub background_color: egui::Color32,
    pub font_id: egui::FontId,
    pub current_line: usize,
}

type LayouterFn<'t> = &'t mut dyn FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley>;

#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct TextEdit<'t> {
    text: &'t mut dyn TextBuffer,
    prefix: Atoms<'static>,
    suffix: Atoms<'static>,
    hint_text: Atoms<'static>,
    id: Option<Id>,
    id_salt: Option<IdSalt>,
    font_selection: FontSelection,
    text_color: Option<Color32>,
    layouter: Option<LayouterFn<'t>>,
    password: bool,
    frame: Option<Frame>,
    margin: Margin,
    multiline: bool,
    interactive: bool,
    desired_width: Option<f32>,
    desired_height_rows: usize,
    event_filter: EventFilter,
    cursor_at_end: bool,
    min_size: Vec2,
    align: Align2,
    clip_text: bool,
    char_limit: usize,
    return_key: Option<KeyboardShortcut>,
    background_color: Option<Color32>,
    gutter_config: Option<GutterConfig>,
    gutter_scroll_x: f32,
    autopair: bool,
}

impl WidgetWithState for TextEdit<'_> {
    type State = TextEditState;
}

impl TextEdit<'_> {
    pub fn load_state(ctx: &Context, id: Id) -> Option<TextEditState> {
        TextEditState::load(ctx, id)
    }

    pub fn store_state(ctx: &Context, id: Id, state: TextEditState) {
        state.store(ctx, id);
    }
}

impl<'t> TextEdit<'t> {
    /// No newlines (`\n`) allowed. Pressing enter key will result in the [`TextEdit`] losing focus (`response.lost_focus`).
    pub fn singleline(text: &'t mut dyn TextBuffer) -> Self {
        Self {
            desired_height_rows: 1,
            multiline: false,
            clip_text: true,
            ..Self::multiline(text)
        }
    }

    /// A [`TextEdit`] for multiple lines. Pressing enter key will create a new line by default (can be changed with [`return_key`](TextEdit::return_key)).
    pub fn multiline(text: &'t mut dyn TextBuffer) -> Self {
        Self {
            text,
            prefix: Default::default(),
            suffix: Default::default(),
            hint_text: Default::default(),
            id: None,
            id_salt: None,
            font_selection: Default::default(),
            text_color: None,
            layouter: None,
            password: false,
            frame: None,
            margin: Margin::symmetric(4, 2),
            multiline: true,
            interactive: true,
            desired_width: None,
            desired_height_rows: 4,
            event_filter: EventFilter {
                // moving the cursor is really important
                horizontal_arrows: true,
                vertical_arrows: true,
                tab: false, // tab is used to change focus, not to insert a tab character
                ..Default::default()
            },
            cursor_at_end: true,
            min_size: Vec2::ZERO,
            align: Align2::LEFT_TOP,
            clip_text: false,
            char_limit: usize::MAX,
            return_key: Some(KeyboardShortcut::new(Modifiers::NONE, Key::Enter)),
            background_color: None,
            gutter_config: None,
            gutter_scroll_x: 0.0,
            autopair: false,
        }
    }

    /// Build a [`TextEdit`] focused on code editing.
    /// By default it comes with:
    /// - monospaced font
    /// - focus lock (tab will insert a tab character instead of moving focus)
    pub fn code_editor(self) -> Self {
        self.font(TextStyle::Monospace)
            .lock_focus(true)
            .autopair(true)
    }

    fn autopair(mut self, on: bool) -> Self {
        self.autopair = on;
        self
    }

    /// Use if you want to set an explicit [`Id`] for this widget.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// A source for the unique [`Id`], e.g. `.id_source("second_text_edit_field")` or `.id_source(loop_index)`.
    #[inline]
    pub fn id_source(self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt(id_salt)
    }

    /// A source for the unique [`Id`], e.g. `.id_salt("second_text_edit_field")` or `.id_salt(loop_index)`.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Show a faint hint text when the text field is empty.
    #[inline]
    pub fn hint_text(mut self, hint_text: impl IntoAtoms<'static>) -> Self {
        self.hint_text = hint_text.into_atoms();
        self
    }

    /// Add a prefix to the text edit. This will always be shown before the editable text.
    #[inline]
    pub fn prefix(mut self, prefix: impl IntoAtoms<'static>) -> Self {
        self.prefix = prefix.into_atoms();
        self
    }

    /// Add a suffix to the text edit. This will always be shown after the editable text.
    #[inline]
    pub fn suffix(mut self, suffix: impl IntoAtoms<'static>) -> Self {
        self.suffix = suffix.into_atoms();
        self
    }

    /// Set the background color of the [`TextEdit`]. The default is [`egui::Visuals::text_edit_bg_color`].
    #[inline]
    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    /// If true, hide the letters from view and prevent copying from the field.
    #[inline]
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Pick a [`egui::FontId`] or [`TextStyle`].
    #[inline]
    pub fn font(mut self, font_selection: impl Into<FontSelection>) -> Self {
        self.font_selection = font_selection.into();
        self
    }

    #[inline]
    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text_color = Some(text_color);
        self
    }

    #[inline]
    pub fn text_color_opt(mut self, text_color: Option<Color32>) -> Self {
        self.text_color = text_color;
        self
    }

    /// Override how text is being shown inside the [`TextEdit`].
    ///
    /// This can be used to implement things like syntax highlighting.
    #[inline]
    pub fn layouter(
        mut self,
        layouter: &'t mut dyn FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley>,
    ) -> Self {
        self.layouter = Some(layouter);

        self
    }

    /// Default is `true`. If set to `false` then you cannot interact with the text (neither edit or select it).
    #[inline]
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Customize the [`Frame`] around the text edit.
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Set margin of text. Default is `Margin::symmetric(4.0, 2.0)`
    #[inline]
    pub fn margin(mut self, margin: impl Into<Margin>) -> Self {
        self.margin = margin.into();
        self
    }

    /// Set to 0.0 to keep as small as possible.
    /// Set to [`f32::INFINITY`] to take up all available space (i.e. disable automatic word wrap).
    #[inline]
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// Set the number of rows to show by default.
    /// The default for singleline text is `1`.
    /// The default for multiline text is `4`.
    #[inline]
    pub fn desired_rows(mut self, desired_height_rows: usize) -> Self {
        self.desired_height_rows = desired_height_rows;
        self
    }

    /// When `false` (default), pressing TAB will move focus
    /// to the next widget.
    ///
    /// When `true`, the widget will keep the focus and pressing TAB
    /// will insert the `'\t'` character.
    #[inline]
    pub fn lock_focus(mut self, tab_will_indent: bool) -> Self {
        self.event_filter.tab = tab_will_indent;
        self
    }

    /// When `true` (default), the cursor will initially be placed at the end of the text.
    ///
    /// When `false`, the cursor will initially be placed at the beginning of the text.
    #[inline]
    pub fn cursor_at_end(mut self, b: bool) -> Self {
        self.cursor_at_end = b;
        self
    }

    /// When `true` (default), overflowing text will be clipped.
    ///
    /// When `false`, widget width will expand to make all text visible.
    ///
    /// This only works for singleline [`TextEdit`].
    #[inline]
    pub fn clip_text(mut self, b: bool) -> Self {
        // always show everything in multiline
        if !self.multiline {
            self.clip_text = b;
        }
        self
    }

    /// Sets the limit for the amount of characters can be entered
    ///
    /// This only works for singleline [`TextEdit`]
    #[inline]
    pub fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    /// Set the horizontal align of the inner text.
    #[inline]
    pub fn horizontal_align(mut self, align: Align) -> Self {
        self.align.0[0] = align;
        self
    }

    /// Set the vertical align of the inner text.
    #[inline]
    pub fn vertical_align(mut self, align: Align) -> Self {
        self.align.0[1] = align;
        self
    }

    /// Set the minimum size of the [`TextEdit`].
    #[inline]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set the return key combination.
    ///
    /// This combination will cause a newline on multiline,
    /// whereas on singleline it will cause the widget to lose focus.
    ///
    /// This combination is optional and can be disabled by passing [`None`] into this function.
    #[inline]
    pub fn return_key(mut self, return_key: impl Into<Option<KeyboardShortcut>>) -> Self {
        self.return_key = return_key.into();
        self
    }

    pub fn show_gutter(mut self, config: GutterConfig) -> Self {
        self.gutter_config = Some(config);
        self
    }

    pub fn gutter_scroll_x(mut self, x: f32) -> Self {
        self.gutter_scroll_x = x;
        self
    }
}

// ----------------------------------------------------------------------------

impl Widget for TextEdit<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response.response
    }
}

impl TextEdit<'_> {
    /// Show the [`TextEdit`], returning a rich [`TextEditOutput`].
    pub fn show(self, ui: &mut Ui) -> TextEditOutput {
        let TextEdit {
            text,
            prefix,
            suffix,
            mut hint_text,
            id,
            id_salt,
            font_selection,
            text_color,
            layouter,
            password,
            frame,
            margin,
            multiline,
            interactive,
            desired_width,
            desired_height_rows,
            event_filter,
            cursor_at_end,
            min_size,
            align,
            clip_text,
            char_limit,
            return_key,
            background_color,
            gutter_config,
            gutter_scroll_x,
            autopair,
        } = self;

        let line_count = if text.as_str().is_empty() {
            1
        } else {
            text.as_str()
                .as_bytes()
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
                + 1
        };
        let gutter_w = gutter_config.as_ref().map_or(0.0, |g| {
            let digit_count = if line_count < 10 {
                1
            } else {
                line_count.ilog10() as usize + 1
            };
            g.padding_left + digit_count as f32 * g.digit_width + g.padding_right
        });

        let text_color = text_color
            .or_else(|| ui.visuals().override_text_color)
            // .unwrap_or_else(|| ui.style().interact(&response).text_color()); // too bright
            .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());

        let hint_text_str = hint_text.text().unwrap_or_default().to_string();

        let font_id = font_selection.resolve(ui.style());
        let row_height = ui.fonts_mut(|f| f.row_height(&font_id));
        const MIN_WIDTH: f32 = 24.0; // Never make a [`TextEdit`] more narrow than this.
        let available_width = ui.available_width().at_least(MIN_WIDTH);
        let desired_width = desired_width
            .unwrap_or_else(|| ui.spacing().text_edit_width)
            .at_least(min_size.x);
        let allocate_width = desired_width.at_most(available_width);

        let font_id_clone = font_id.clone();
        let mut default_layouter = move |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
            let text = mask_if_password(password, text.as_str());
            let mut layout_job = if multiline {
                LayoutJob::simple(text, font_id_clone.clone(), text_color, wrap_width)
            } else {
                LayoutJob::simple_singleline(text, font_id_clone.clone(), text_color)
            };
            layout_job.halign = align.x();
            // We want to keep the trailing whitespace, since hiding it feels really weird when typing
            layout_job.keep_trailing_whitespace = true;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        let layouter = layouter.unwrap_or(&mut default_layouter);

        let min_inner_height = (desired_height_rows.at_least(1) as f32) * row_height;

        let id = id.unwrap_or_else(|| {
            if let Some(id_salt) = id_salt {
                ui.make_persistent_id(id_salt)
            } else {
                // Since we are only storing the cursor a persistent Id is not super important
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                id
            }
        });

        // On touch screens (e.g. mobile in `eframe` web), should
        // dragging select text, or scroll the enclosing [`ScrollArea`] (if any)?
        // Since currently copying selected text in not supported on `eframe` web,
        // we prioritize touch-scrolling:
        let allow_drag_to_select =
            ui.input(|i| !i.has_touch_screen()) || ui.memory(|mem| mem.has_focus(id));

        let sense = if interactive {
            if allow_drag_to_select {
                Sense::click_and_drag()
            } else {
                Sense::click()
            }
        } else {
            Sense::hover()
        };

        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let mut cursor_range = None;
        let mut prev_cursor_range = None;

        let mut text_changed = false;
        let text_mutable = text.is_mutable();

        let mut handle_events = |ui: &Ui, galley: &mut Arc<Galley>, layouter, wrap_width, text| {
            if interactive && ui.memory(|mem| mem.has_focus(id)) {
                ui.memory_mut(|mem| mem.set_focus_lock_filter(id, event_filter));

                let default_cursor_range = if cursor_at_end {
                    CCursorRange::one(galley.end())
                } else {
                    CCursorRange::default()
                };
                prev_cursor_range = state.cursor.range(galley);

                let (changed, new_cursor_range) = events(
                    ui,
                    &mut state,
                    text,
                    galley,
                    layouter,
                    id,
                    wrap_width,
                    multiline,
                    password,
                    default_cursor_range,
                    char_limit,
                    event_filter,
                    return_key,
                    autopair,
                );

                if changed {
                    text_changed = true;
                }
                cursor_range = Some(new_cursor_range);
            }
        };

        // We need to calculate the galley within the atom closure, so we can calculate it based on
        // the available width (in case of wrapping multiline text edits). But we show it later,
        // so we can clip it to the available size. Thus, extract it from the atom closure here.
        let mut get_galley = None;
        let inner_rect_id = Id::new("text_edit_rect");
        let mut response = {
            let any_shrink = hint_text.any_shrink();
            // Ideally we could just do `let mut atoms = prefix` here, but that won't compile
            // but due to servo/rust-smallvec#146 (also see the comment below).
            let mut atoms: Atoms<'_> = Atoms::new(());

            // TODO(servo/rust-smallvec#146): Use extend_right instead of the loop once we have
            // smallvec 2.0. Using `extend_right` here won't compile, due to lifetime issues.
            for atom in prefix {
                atoms.push_right(atom);
            }

            if text.as_str().is_empty() && !hint_text.is_empty() {
                // Add hint_text (if any):
                let mut shrunk = any_shrink;
                let mut first = true;

                // Since we can't set a fallback color per atom, we have to override it here.
                // Sucks, since it means users won't be able to override it.
                hint_text.map_texts(|t| t.color(ui.style().visuals.weak_text_color()));

                for mut atom in hint_text {
                    if !shrunk && matches!(atom.kind, AtomKind::Text(_)) {
                        // elide the hint_text if needed
                        atom = atom.atom_shrink(true);
                        atom = atom.atom_grow(true);
                        shrunk = true;
                    }

                    if first {
                        // The first atom in the hint text gets inner_rect_id, so we can know
                        // where to paint the cursor
                        atom = atom.atom_id(inner_rect_id);
                        first = false;
                    }

                    // The hint text should be shown left top instead of centered (important for
                    // multi line text edits)
                    atoms.push_right(atom.atom_align(Align2::LEFT_TOP));
                }

                // Calculate the empty galley, so it can be read later. The available width is
                // technically wrong, but doesn't matter since the galley is empty
                let available_width = allocate_width - margin.sum().x;
                let galley = layouter(ui, text, available_width);

                // We can't update the galley immediately here, since it would show both hint text
                // and the newly typed letter. So we pass a clone instead, and accept having a frame
                // delay on the very first keystroke.
                let mut galley_clone = Arc::clone(&galley);
                handle_events(ui, &mut galley_clone, layouter, available_width, text);

                get_galley = Some(galley);
            } else {
                // We need to shrink when clip_text, so that we don't exceed the available size
                // and thus clip. We also need to shrink in multi line text edits, so text can
                // wrap appropriately.
                let should_shrink = clip_text || multiline;

                // We need a closure here, so we can calculate the galley based on the available
                // width (after adding suffix and prefix), for correct wrapping in multi line text
                // edits
                atoms.push_right(
                    AtomKind::closure(|ui, args| {
                        let mut galley = layouter(ui, text, args.available_size.x);

                        // Handling events here allows us to update the galley immediately on
                        // keystrokes, avoiding frame delays, and ensuring the scroll_to within
                        // ScrollAreas works correctly.
                        handle_events(ui, &mut galley, layouter, args.available_size.x, text);

                        let intrinsic_size = galley.intrinsic_size();
                        let mut size = galley.size();
                        size.y = size.y.at_least(min_inner_height);
                        if clip_text {
                            size.x = size.x.at_most(args.available_size.x);
                        }

                        // We paint the galley later, so we can do clipping and offsetting
                        get_galley = Some(galley);
                        IntoSizedResult {
                            intrinsic_size,
                            sized: SizedAtomKind::Empty { size: Some(size) },
                        }
                    })
                    .atom_grow(true)
                    .atom_align(align)
                    .atom_id(inner_rect_id)
                    .atom_shrink(should_shrink),
                );
            }

            // TODO(servo/rust-smallvec#146): Use extend_right instead of the loop once we have
            // smallvec 2.0. Using `extend_right` here won't compile, due to lifetime issues.
            for atom in suffix {
                atoms.push_right(atom);
            }

            let custom_frame = frame.is_some();
            let mut frame = frame.unwrap_or_else(|| Frame::new().inner_margin(margin));
            if gutter_w > 0.0 {
                frame.inner_margin.left = frame
                    .inner_margin
                    .left
                    .saturating_add(gutter_w.round() as i8);
            }

            let min_height = min_size.y.max(min_inner_height) + frame.total_margin().sum().y;

            // This wrap mode only affects the hint_text
            let wrap_mode = if multiline {
                TextWrapMode::Wrap
            } else {
                TextWrapMode::Truncate
            };

            let mut allocated = AtomLayout::new(atoms)
                .id(id)
                .min_size(Vec2::new(allocate_width, min_height))
                .max_width(allocate_width)
                .sense(sense)
                .frame(frame)
                .align2(align)
                .wrap_mode(wrap_mode)
                .allocate(ui);

            allocated.frame = if custom_frame {
                allocated.frame
            } else {
                let visuals = ui.style().interact(&allocated.response);
                let background_color =
                    background_color.unwrap_or_else(|| ui.visuals().text_edit_bg_color());

                let (corner_radius, background_color, stroke) = if text_mutable {
                    if allocated.response.has_focus() {
                        (
                            visuals.corner_radius,
                            background_color,
                            ui.visuals().selection.stroke,
                        )
                    } else {
                        (visuals.corner_radius, background_color, visuals.bg_stroke)
                    }
                } else {
                    let visuals = &ui.style().visuals.widgets.inactive;
                    (
                        visuals.corner_radius,
                        Color32::TRANSPARENT,
                        visuals.bg_stroke,
                    )
                };
                allocated
                    .frame
                    .fill(background_color)
                    .corner_radius(corner_radius)
                    .inner_margin(
                        allocated.frame.inner_margin
                            + Margin::same((visuals.expansion - stroke.width).round() as i8),
                    )
                    .outer_margin(Margin::same(-(visuals.expansion as i8)))
                    .stroke(stroke)
            };

            allocated.paint(ui)
        };

        let inner_rect = response.rect(inner_rect_id).unwrap_or(Rect::ZERO);

        // Our atom closure was now called, so the galley should always be available here
        let mut galley = get_galley.expect("Galley should be available here");

        // Don't send `OutputEvent::Clicked` when a user presses the space bar
        response.flags -= response::Flags::FAKE_PRIMARY_CLICKED;
        let pinned_text_left = inner_rect.left() + gutter_scroll_x;
        let text_clip_rect = if gutter_w > 0.0 {
            Rect::from_min_max(pos2(pinned_text_left, inner_rect.top()), inner_rect.max)
        } else {
            inner_rect
        };
        let painter = ui.painter_at(text_clip_rect.expand(1.0)); // expand to avoid clipping cursor

        if interactive && let Some(pointer_pos) = response.interact_pointer_pos() {
            if response.hovered() && text.is_mutable() {
                ui.output_mut(|o| o.mutable_text_under_cursor = true);
            }

            // TODO(emilk): drag selected text to either move or clone (ctrl on windows, alt on mac)

            let cursor_at_pointer = galley.cursor_from_pos(
                pointer_pos - inner_rect.min + state.text_offset + vec2(galley.rect.left(), 0.0),
            );

            if ui.visuals().text_cursor.preview
                && response.hovered()
                && ui.input(|i| i.pointer.is_moving())
            {
                // text cursor preview:
                let cursor_rect = TSTransform::from_translation(
                    inner_rect.min.to_vec2() - vec2(galley.rect.left(), 0.0),
                ) * cursor_rect(&galley, &cursor_at_pointer, row_height);
                text_selection::visuals::paint_cursor_end(&painter, ui.visuals(), cursor_rect);
            }

            let is_being_dragged = ui.is_being_dragged(response.id);
            let did_interact = state.cursor.pointer_interaction(
                ui,
                &response,
                cursor_at_pointer,
                &galley,
                is_being_dragged,
            );

            if did_interact || response.clicked() {
                ui.memory_mut(|mem| mem.request_focus(response.id));

                state.last_interaction_time = ui.input(|i| i.time);
            }
        }

        if interactive && response.hovered() {
            ui.set_cursor_icon(CursorIcon::Text);
        }

        if text_changed {
            response.mark_changed();
        }

        let mut galley_pos = align
            .align_size_within_rect(galley.size(), inner_rect)
            .intersect(inner_rect) // limit pos to the response rect area
            .min;
        let align_offset = inner_rect.left_top() - galley_pos;

        // Visual clipping for singleline text editor with text larger than width
        if clip_text && align_offset.x == 0.0 {
            let cursor_pos = match (cursor_range, ui.memory(|mem| mem.has_focus(id))) {
                (Some(cursor_range), true) => galley.pos_from_cursor(cursor_range.primary).min.x,
                _ => 0.0,
            };

            let mut offset_x = state.text_offset.x;
            let visible_range = offset_x..=offset_x + inner_rect.width();

            if !visible_range.contains(&cursor_pos) {
                if cursor_pos < *visible_range.start() {
                    offset_x = cursor_pos;
                } else {
                    offset_x = cursor_pos - inner_rect.width();
                }
            }

            offset_x = offset_x
                .at_most(galley.size().x - inner_rect.width())
                .at_least(0.0);

            state.text_offset = vec2(offset_x, align_offset.y);
            galley_pos -= vec2(offset_x, 0.0);
        } else {
            state.text_offset = align_offset;
        }

        let current_row_idx = state
            .cursor
            .range(&galley)
            .map(|r| galley.layout_from_cursor(r.primary).row)
            .unwrap_or_else(|| {
                gutter_config
                    .as_ref()
                    .map_or(0, |g| g.current_line.saturating_sub(1))
            });

        let selection_changed = if let (Some(cursor_range), Some(prev_cursor_range)) =
            (cursor_range, prev_cursor_range)
        {
            prev_cursor_range != cursor_range
        } else {
            false
        };

        // `paint_ime_preedit_text_visuals` is internal to egui, so we always paint IME
        // composition via the selection/text-cursor path (the "legacy" visuals).
        let should_paint_ime_visuals_the_legacy_way = true;

        if ui.is_rect_visible(inner_rect) {
            let has_focus = ui.memory(|mem| mem.has_focus(id));

            if has_focus
                && (state.cursor_purpose.is_selection() || should_paint_ime_visuals_the_legacy_way)
                && let Some(cursor_range) = state.cursor.range(&galley)
            {
                // Add text selection rectangles to the galley:
                paint_text_selection(&mut galley, ui.visuals(), &cursor_range, None);
            }

            if let Some(ref gutter) = gutter_config {
                let pinned_text_left = inner_rect.left() + gutter_scroll_x;
                let gutter_rect = Rect::from_min_max(
                    pos2(ui.clip_rect().left(), response.rect.top()),
                    pos2(
                        pinned_text_left - 1.0,
                        response.rect.bottom().max(response.rect.top()),
                    ),
                );
                ui.painter()
                    .rect_filled(gutter_rect, 0.0, gutter.background_color);

                let row = galley.rows.get(current_row_idx);
                let row_y = row.map_or(galley_pos.y, |row| galley_pos.y + row.pos.y);
                let bar_rect = Rect::from_min_max(
                    pos2(ui.clip_rect().left(), row_y),
                    pos2(response.rect.right(), row_y + row_height),
                );
                ui.painter().rect_filled(
                    bar_rect,
                    0.0,
                    gutter.current_line_color.gamma_multiply(0.14),
                );
            }

            painter.galley(
                galley_pos - vec2(galley.rect.left(), 0.0),
                Arc::clone(&galley),
                text_color,
            );

            if has_focus && let Some(cursor_range) = state.cursor.range(&galley) {
                let primary_cursor_rect = cursor_rect(&galley, &cursor_range.primary, row_height)
                    .translate(galley_pos.to_vec2() - vec2(galley.rect.left(), 0.0));

                if response.changed() || selection_changed {
                    // Scroll to keep primary cursor in view:
                    ui.scroll_to_rect(primary_cursor_rect, None);
                }

                if text.is_mutable() && interactive {
                    let now = ui.input(|i| i.time);
                    if response.changed() || selection_changed {
                        state.last_interaction_time = now;
                    }

                    // Only show (and blink) cursor if the egui viewport has focus.
                    let viewport_has_focus = ui.input(|i| i.focused);
                    if viewport_has_focus {
                        let time_since_last_interaction = now - state.last_interaction_time;
                        if should_paint_ime_visuals_the_legacy_way
                            || state.cursor_purpose.is_selection()
                        {
                            text_selection::visuals::paint_text_cursor(
                                ui,
                                &painter,
                                primary_cursor_rect,
                                time_since_last_interaction,
                            );
                        }
                    }
                    if ui.memory(|mem| mem.owns_ime_events(id)) {
                        // Set IME output (in screen coords) when text is editable and visible
                        let to_global = ui
                            .ctx()
                            .layer_transform_to_global(ui.layer_id())
                            .unwrap_or_default();
                        ui.output_mut(|o| {
                            o.ime = Some(egui::output::IMEOutput {
                                purpose: if password {
                                    egui::IMEPurpose::Password
                                } else {
                                    egui::IMEPurpose::Normal
                                },
                                rect: to_global * inner_rect,
                                cursor_rect: to_global * primary_cursor_rect,
                                should_interrupt_composition: false,
                            });
                        });
                    }
                }
            }
        }

        if let Some(ref gutter) = gutter_config {
            let gutter_painter = ui.painter();
            let pinned_text_left = inner_rect.left() + gutter_scroll_x;

            for (i, row) in galley.rows.iter().enumerate() {
                let line_num = i + 1;
                let line_y = galley_pos.y + row.pos.y;
                let is_current = line_num == current_row_idx + 1;
                let color = if is_current {
                    gutter.current_line_color
                } else {
                    gutter.muted_color
                };
                let x = pinned_text_left - gutter.line_number_right_offset;
                gutter_painter.text(
                    pos2(x, line_y),
                    Align2::RIGHT_TOP,
                    line_num.to_string(),
                    gutter.font_id.clone(),
                    color,
                );
            }
        }

        state.clone().store(ui.ctx(), id);

        if response.changed() {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    ui.is_enabled(),
                    mask_if_password(password, text.as_str()),
                    mask_if_password(password, text.as_str()),
                    hint_text_str.as_str(),
                )
            });
        } else if selection_changed && let Some(cursor_range) = cursor_range {
            let char_range = cursor_range.as_sorted_char_range();
            let info = WidgetInfo::text_selection_changed(
                ui.is_enabled(),
                char_range,
                mask_if_password(password, text.as_str()),
            );
            response.output_event(OutputEvent::TextSelectionChanged(info));
        } else {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    ui.is_enabled(),
                    mask_if_password(password, text.as_str()),
                    mask_if_password(password, text.as_str()),
                    hint_text_str.as_str(),
                )
            });
        }

        let role = if password {
            accesskit::Role::PasswordInput
        } else if multiline {
            accesskit::Role::MultilineTextInput
        } else {
            accesskit::Role::TextInput
        };

        egui::text_selection::accesskit_text::update_accesskit_for_text_widget(
            ui.ctx(),
            id,
            cursor_range,
            role,
            TSTransform::from_translation(galley_pos.to_vec2()),
            &galley,
        );

        TextEditOutput {
            response,
            galley,
            galley_pos,
            text_clip_rect,
            state,
            cursor_range,
        }
    }
}

fn mask_if_password(is_password: bool, text: &str) -> String {
    fn mask_password(text: &str) -> String {
        std::iter::repeat_n(
            epaint::text::PASSWORD_REPLACEMENT_CHAR,
            text.chars().count(),
        )
        .collect::<String>()
    }

    if is_password {
        mask_password(text)
    } else {
        text.to_owned()
    }
}

// ----------------------------------------------------------------------------

fn indent_of_previous_line(text: &str, insert_index: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if insert_index == 0 {
        return String::new();
    }
    let mut line_start = 0;
    let mut i = 0;
    while i < insert_index.saturating_sub(1) {
        if chars[i] == '\n' {
            line_start = i + 1;
        }
        i += 1;
    }
    let mut indent_len = 0;
    for &ch in &chars[line_start..insert_index.saturating_sub(1)] {
        if matches!(ch, ' ' | '\t') {
            indent_len += 1;
        } else {
            break;
        }
    }
    chars[line_start..line_start + indent_len].iter().collect()
}

/// Check for (keyboard) events to edit the cursor and/or text.
#[expect(clippy::too_many_arguments)]
fn events(
    ui: &Ui,
    state: &mut TextEditState,
    text: &mut dyn TextBuffer,
    galley: &mut Arc<Galley>,
    layouter: &mut dyn FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley>,
    id: Id,
    wrap_width: f32,
    multiline: bool,
    password: bool,
    default_cursor_range: CCursorRange,
    char_limit: usize,
    event_filter: EventFilter,
    return_key: Option<KeyboardShortcut>,
    autopair: bool,
) -> (bool, CCursorRange) {
    let os = ui.os();

    let mut cursor_range = state.cursor.range(galley).unwrap_or(default_cursor_range);

    // We feed state to the undoer both before and after handling input
    // so that the undoer creates automatic saves even when there are no events for a while.
    state.undoer.lock().feed_state(
        ui.input(|i| i.time),
        &(cursor_range, text.as_str().to_owned()),
    );

    let copy_if_not_password = |ui: &Ui, text: String| {
        if !password {
            ui.copy_text(text);
        }
    };

    let mut any_change = false;

    let events = ui.input(|i| i.filtered_events(&event_filter));

    let owns_ime_events = ui.memory(|mem| mem.owns_ime_events(id));
    if !owns_ime_events {
        state.cursor_purpose = TextEditCursorPurpose::Selection;
    }

    enum CursorMutation {
        Selection(CCursorRange),
        ImeComposition {
            cursor_range: CCursorRange,
            active_range: Option<std::ops::Range<CCursor>>,
        },
        ImeCompositionCursorRange(CCursorRange),
    }

    for event in &events {
        let did_mutate_text = match event {
            // First handle events that only changes the selection cursor, not the text:
            event if cursor_range.on_event(os, event, galley, id) => None,

            Event::Copy => {
                if !cursor_range.is_empty() {
                    copy_if_not_password(ui, cursor_range.slice_str(text.as_str()).to_owned());
                }
                None
            }
            Event::Cut => {
                if cursor_range.is_empty() {
                    None
                } else {
                    copy_if_not_password(ui, cursor_range.slice_str(text.as_str()).to_owned());
                    Some(CursorMutation::Selection(CCursorRange::one(
                        text.delete_selected(&cursor_range),
                    )))
                }
            }
            Event::Paste(text_to_insert) => {
                if text_to_insert.is_empty() {
                    None
                } else {
                    let mut ccursor = text.delete_selected(&cursor_range);
                    if multiline {
                        text.insert_text_at(&mut ccursor, text_to_insert, char_limit);
                    } else {
                        let single_line = text_to_insert.replace(['\r', '\n'], " ");
                        text.insert_text_at(&mut ccursor, &single_line, char_limit);
                    }

                    Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                }
            }
            Event::Text(text_to_insert) => {
                // Newlines are handled by `Key::Enter`.
                if !text_to_insert.is_empty() && text_to_insert != "\n" && text_to_insert != "\r" {
                    let mut ccursor = text.delete_selected(&cursor_range);

                    if autopair && text_to_insert.chars().count() == 1 {
                        let c = text_to_insert.chars().next().unwrap();
                        let (pair_char, is_opening) = match c {
                            '(' => (Some(')'), true),
                            '[' => (Some(']'), true),
                            '{' => (Some('}'), true),
                            '"' => (Some('"'), true),
                            '\'' => (Some('\''), true),
                            '`' => (Some('`'), true),
                            ')' => (Some(')'), false),
                            ']' => (Some(']'), false),
                            '}' => (Some('}'), false),
                            _ => (None, false),
                        };
                        if let Some(pair_char) = pair_char {
                            let next = text.as_str().chars().nth(usize::from(ccursor.index));
                            let overtype = !matches!(c, '(' | '[' | '{') && next == Some(pair_char);
                            if overtype {
                                Some(CursorMutation::Selection(CCursorRange::one(CCursor::new(
                                    usize::from(ccursor.index + 1),
                                ))))
                            } else if is_opening {
                                text.insert_text_at(&mut ccursor, text_to_insert, char_limit);
                                let between = ccursor;
                                text.insert_text_at(
                                    &mut ccursor,
                                    &pair_char.to_string(),
                                    char_limit,
                                );
                                Some(CursorMutation::Selection(CCursorRange::one(between)))
                            } else {
                                text.insert_text_at(&mut ccursor, text_to_insert, char_limit);
                                Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                            }
                        } else {
                            text.insert_text_at(&mut ccursor, text_to_insert, char_limit);
                            Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                        }
                    } else {
                        text.insert_text_at(&mut ccursor, text_to_insert, char_limit);
                        Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                    }
                } else {
                    None
                }
            }
            Event::Key {
                key: Key::Tab,
                pressed: true,
                modifiers,
                ..
            } if multiline => {
                let mut ccursor = text.delete_selected(&cursor_range);
                if modifiers.shift {
                    // TODO(emilk): support removing indentation over a selection?
                    text.decrease_indentation(&mut ccursor);
                } else {
                    text.insert_text_at(&mut ccursor, "\t", char_limit);
                }
                Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if return_key.is_some_and(|return_key| {
                *key == return_key.logical_key && modifiers.matches_logically(return_key.modifiers)
            }) =>
            {
                if multiline {
                    let mut ccursor = text.delete_selected(&cursor_range);
                    text.insert_text_at(&mut ccursor, "\n", char_limit);
                    if autopair {
                        let indent =
                            indent_of_previous_line(text.as_str(), usize::from(ccursor.index));
                        if !indent.is_empty() {
                            text.insert_text_at(&mut ccursor, &indent, char_limit);
                        }
                    }
                    Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                } else {
                    ui.memory_mut(|mem| mem.surrender_focus(id)); // End input with enter
                    break;
                }
            }

            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if (modifiers.matches_logically(Modifiers::COMMAND) && *key == Key::Y)
                || (modifiers.matches_logically(Modifiers::SHIFT | Modifiers::COMMAND)
                    && *key == Key::Z) =>
            {
                if let Some((redo_ccursor_range, redo_txt)) = state
                    .undoer
                    .lock()
                    .redo(&(cursor_range, text.as_str().to_owned()))
                {
                    text.replace_with(redo_txt);
                    Some(CursorMutation::Selection(*redo_ccursor_range))
                } else {
                    None
                }
            }

            Event::Key {
                key: Key::Z,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_logically(Modifiers::COMMAND) => {
                if let Some((undo_ccursor_range, undo_txt)) = state
                    .undoer
                    .lock()
                    .undo(&(cursor_range, text.as_str().to_owned()))
                {
                    text.replace_with(undo_txt);
                    Some(CursorMutation::Selection(*undo_ccursor_range))
                } else {
                    None
                }
            }

            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => check_for_mutating_key_press(os, &cursor_range, text, galley, modifiers, *key)
                .map(CursorMutation::Selection),

            Event::Ime(ime_event) if owns_ime_events => {
                /// Both `ImeEvent::Preedit("")` and `ImeEvent::Commit("")`
                /// might be emitted from different integrations to signify that
                /// the current IME composition should be cleared.
                fn clear_preedit_text(
                    text: &mut dyn TextBuffer,
                    preedit_range: &CCursorRange,
                ) -> CCursor {
                    text.delete_selected(preedit_range)
                }

                match ime_event {
                    #[expect(deprecated)]
                    ImeEvent::Enabled | ImeEvent::Disabled => None,
                    // Ignore `Preedit`/`Commit` events with empty text when
                    // there is no active IME composition.
                    ImeEvent::Preedit {
                        text: composition_text,
                        ..
                    }
                    | ImeEvent::Commit(composition_text)
                        if composition_text.is_empty()
                            && !state.cursor_purpose.is_ime_composition() =>
                    {
                        None
                    }
                    ImeEvent::Preedit {
                        text: composition_text,
                        ..
                    }
                    | ImeEvent::Commit(composition_text)
                        if composition_text == "\n" || composition_text == "\r" =>
                    {
                        None
                    }
                    ImeEvent::Preedit {
                        text: preedit_text,
                        active_range_chars,
                    } => {
                        let mut ccursor = clear_preedit_text(text, &cursor_range);

                        if preedit_text.is_empty() {
                            Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                        } else {
                            let start_cursor = ccursor;
                            text.insert_text_at(&mut ccursor, preedit_text, char_limit);
                            Some(CursorMutation::ImeComposition {
                                cursor_range: CCursorRange::two(start_cursor, ccursor),
                                active_range: active_range_chars.clone().map(|range| {
                                    CCursor::new(range.start)..CCursor::new(range.end)
                                }),
                            })
                        }
                    }
                    ImeEvent::Commit(commit_text) => {
                        let mut ccursor = clear_preedit_text(text, &cursor_range);

                        if !commit_text.is_empty() {
                            text.insert_text_at(&mut ccursor, commit_text, char_limit);
                        }

                        Some(CursorMutation::Selection(CCursorRange::one(ccursor)))
                    }
                    ImeEvent::DeleteSurrounding {
                        before_chars,
                        after_chars,
                    } => Some(CursorMutation::ImeCompositionCursorRange(
                        text.delete_surrounding_chars(cursor_range, *before_chars, *after_chars),
                    )),
                }
            }

            _ => None,
        };

        if let Some(cursor_mutation) = did_mutate_text {
            any_change = true;

            // Layout again to avoid frame delay, and to keep `text` and `galley` in sync.
            *galley = layouter(ui, text, wrap_width);

            // Set cursor_range using new galley:
            match cursor_mutation {
                CursorMutation::Selection(new_cursor_range) => {
                    cursor_range = new_cursor_range;
                    state.cursor_purpose = TextEditCursorPurpose::Selection;
                }
                CursorMutation::ImeComposition {
                    cursor_range: new_cursor_range,
                    active_range,
                } => {
                    cursor_range = new_cursor_range;
                    state.cursor_purpose = TextEditCursorPurpose::ImeComposition { active_range };
                }
                CursorMutation::ImeCompositionCursorRange(new_cursor_range) => {
                    cursor_range = new_cursor_range;
                }
            }
        }
    }

    state.cursor.set_char_range(Some(cursor_range));

    state.undoer.lock().feed_state(
        ui.input(|i| i.time),
        &(cursor_range, text.as_str().to_owned()),
    );

    (any_change, cursor_range)
}

// ----------------------------------------------------------------------------

/// Returns `Some(new_cursor)` if we did mutate `text`.
fn check_for_mutating_key_press(
    os: OperatingSystem,
    cursor_range: &CCursorRange,
    text: &mut dyn TextBuffer,
    galley: &Galley,
    modifiers: &Modifiers,
    key: Key,
) -> Option<CCursorRange> {
    match key {
        Key::Backspace => {
            let ccursor = if modifiers.mac_cmd {
                text.delete_paragraph_before_cursor(galley, cursor_range)
            } else if let Some(cursor) = cursor_range.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    text.delete_previous_word(cursor)
                } else {
                    text.delete_previous_char(cursor)
                }
            } else {
                text.delete_selected(cursor_range)
            };
            Some(CCursorRange::one(ccursor))
        }

        Key::Delete if !modifiers.shift || os != OperatingSystem::Windows => {
            let ccursor = if modifiers.mac_cmd {
                text.delete_paragraph_after_cursor(galley, cursor_range)
            } else if let Some(cursor) = cursor_range.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    text.delete_next_word(cursor)
                } else {
                    text.delete_next_char(cursor)
                }
            } else {
                text.delete_selected(cursor_range)
            };
            let ccursor = CCursor {
                prefer_next_row: true,
                ..ccursor
            };
            Some(CCursorRange::one(ccursor))
        }

        Key::H if modifiers.ctrl => {
            let ccursor = text.delete_previous_char(cursor_range.primary);
            Some(CCursorRange::one(ccursor))
        }

        Key::K if modifiers.ctrl => {
            let ccursor = text.delete_paragraph_after_cursor(galley, cursor_range);
            Some(CCursorRange::one(ccursor))
        }

        Key::U if modifiers.ctrl => {
            let ccursor = text.delete_paragraph_before_cursor(galley, cursor_range);
            Some(CCursorRange::one(ccursor))
        }

        Key::W if modifiers.ctrl => {
            let ccursor = if let Some(cursor) = cursor_range.single() {
                text.delete_previous_word(cursor)
            } else {
                text.delete_selected(cursor_range)
            };
            Some(CCursorRange::one(ccursor))
        }

        _ => None,
    }
}

#[cfg(test)]
mod indent_tests {
    use super::indent_of_previous_line;

    #[test]
    fn basic() {
        let text = "    hello\n";
        assert_eq!(indent_of_previous_line(text, 10), "    ");
    }

    #[test]
    fn two_levels() {
        let text = "        hello\n";
        assert_eq!(indent_of_previous_line(text, 14), "        ");
    }

    #[test]
    fn tabs() {
        let text = "\t\thello\n";
        assert_eq!(indent_of_previous_line(text, 8), "\t\t");
    }

    #[test]
    fn no_indent() {
        let text = "hello\n";
        assert_eq!(indent_of_previous_line(text, 6), "");
    }

    #[test]
    fn mixed() {
        let text = "  \t  hello\n";
        assert_eq!(indent_of_previous_line(text, 11), "  \t  ");
    }

    #[test]
    fn empty_is_empty() {
        assert_eq!(indent_of_previous_line("", 0), "");
    }

    #[test]
    fn indents_from_correct_previous_line() {
        let text = "a\n    b";
        assert_eq!(indent_of_previous_line(text, 7), "    ");
    }
}
