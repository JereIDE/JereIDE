{
        {
            // Layout + render.
            let (w, h, _, _) = crate::window::get_window_size();
            let width = w as f64;
            let height = h as f64;
            let status_h = style.font_height + style.padding_y * 2.0;
            let sidebar_w = if subsystems.has_sidebar() && sidebar_visible {
                sidebar_width
            } else {
                0.0
            };

            let tab_h = if !single_file_mode && !docs.is_empty() {
                style.font_height + style.padding_y * 3.0
            } else {
                0.0
            };
            let terminal_h = if subsystems.has_terminal() && terminal.visible {
                terminal_h_override
                    .unwrap_or(
                        (height * 0.3)
                            .min(height - tab_h - status_h - 50.0)
                            .max(80.0),
                    )
                    .min(height - tab_h - status_h - 50.0)
                    .max(80.0)
            } else {
                0.0
            };
            let minimap_w = if minimap_visible { 120.0 } else { 0.0 };
            let breadcrumb_h = if docs.get(active_tab).is_some() {
                style.font_height + style.padding_y * 0.5
            } else {
                0.0
            };
            let content_rect = crate::editor::types::Rect {
                x: sidebar_w,
                y: tab_h + breadcrumb_h,
                w: width - sidebar_w - minimap_w,
                h: height - tab_h - breadcrumb_h - terminal_h - status_h,
            };
            empty_view.set_rect(content_rect);
            // Note-Anvil keeps the markdown preview pinned on for every
            // doc — it's not toggleable in notes mode.
            if subsystems.has_notes_mode() {
                for d in docs.iter_mut() {
                    d.preview.enabled = true;
                }
            }
            if let Some(doc) = docs.get_mut(active_tab) {
                if doc.preview.enabled {
                    // Split the content area into editor | preview panes at the
                    // user-adjustable `preview_split` fraction (drag the divider
                    // to resize; persisted per app). The editor keeps float
                    // rects (its existing wrap/click math has always tolerated
                    // them); the preview rect is snapped to integer pixels so
                    // the background fill and clip rect enclose every logical
                    // pixel. Without snapping, `draw_rect`'s i32 cast truncates
                    // the bottom of the fill, leaving a stale pixel row that
                    // reads as a thin blue line from a previously drawn heading
                    // rule.
                    let half_w = (content_rect.w * preview_split).floor();
                    let left = crate::editor::types::Rect {
                        x: content_rect.x,
                        y: content_rect.y,
                        w: half_w,
                        h: content_rect.h,
                    };
                    let right_x = (content_rect.x + half_w).round();
                    let right_y = content_rect.y.floor();
                    let right_bottom = (content_rect.y + content_rect.h).ceil();
                    let right_right = (content_rect.x + content_rect.w).ceil();
                    let right = crate::editor::types::Rect {
                        x: right_x,
                        y: right_y,
                        w: right_right - right_x,
                        h: right_bottom - right_y,
                    };
                    doc.view.set_rect(left);
                    doc.preview.rect = right;
                } else {
                    doc.view.set_rect(content_rect);
                    doc.preview.rect = crate::editor::types::Rect::default();
                }
            }
            status_view.set_rect(crate::editor::types::Rect {
                x: 0.0,
                y: height - status_h,
                w: width,
                h: status_h,
            });

            let uctx = UpdateContext {
                dt: 1.0 / fps,
                window_width: width,
                window_height: height,
            };
            empty_view.update(&uctx);
            if let Some(doc) = docs.get_mut(active_tab) {
                let dv = &mut doc.view;
                if let Some(buf_id) = dv.buffer_id {
                    use crate::editor::view::DrawContext as _;
                    let line_count =
                        buffer::with_buffer(buf_id, |b| Ok(b.lines.len())).unwrap_or(1);
                    let digits = format!("{}", line_count).len().max(2);
                    let char_w = draw_ctx.font_width(style.code_font, "9");
                    dv.gutter_width = char_w * digits as f64 + style.padding_x * 2.0;
                    dv.code_char_w = char_w;
                }
                dv.update(&uctx);
            }
            status_view.update(&uctx);

            // Autoreload: check for external file changes.
            let changed_paths = autoreload.poll_changed();
            for changed in &changed_paths {
                // Canonicalize to match doc paths.
                let canonical = std::fs::canonicalize(changed)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| changed.clone());
                for doc in docs.iter_mut() {
                    let doc_canon = std::fs::canonicalize(&doc.path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| doc.path.clone());
                    if doc_canon != canonical {
                        continue;
                    }
                    let Some(buf_id) = doc.view.buffer_id else {
                        break;
                    };
                    let path = doc.path.clone();
                    // We watch the parent directory, so our own writes --
                    // notably notes-mode autosave -- echo back as change
                    // events too. A watcher event is only a hint to check;
                    // the authoritative test is whether the bytes on disk
                    // differ from what we last persisted. Read the file and
                    // compare its signature against the one recorded at our
                    // last save: if they match, this is the echo of our own
                    // write, so there is nothing to reload or warn about.
                    let mut disk_state = buffer::default_buffer_state();
                    if buffer::load_file(&mut disk_state, &path).is_err() {
                        break;
                    }
                    let disk_sig = buffer::content_signature(&disk_state.lines);
                    if disk_sig == doc.saved_signature {
                        break;
                    }
                    // The bytes on disk genuinely differ from our last save.
                    if doc_is_modified(doc) {
                        // Local edits would be lost by an automatic reload.
                        nag = Nag::ReloadFromDisk { path };
                    } else {
                        // No local edits: adopt the external content. We
                        // already loaded it above, so move it straight in.
                        let _ = buffer::with_buffer_mut(buf_id, |b| {
                            b.lines = disk_state.lines;
                            // `default_buffer_state()` resets change_id to 1;
                            // a just-opened buffer also sits at 1, so the
                            // doc-view render cache would hit on stale lines.
                            // Bump past the current value to invalidate every
                            // downstream cache.
                            b.change_id = b.change_id.wrapping_add(1).max(1);
                            Ok(())
                        });
                        // Force the render cache to rebuild next frame rather
                        // than relying on the change_id comparison to catch
                        // the bump.
                        doc.cached_change_id = -1;
                        doc.cached_render = std::sync::Arc::new(Vec::new());
                        // Realign the "saved" markers with what is now on
                        // disk so the next external change is judged against
                        // the correct baseline.
                        if let Ok((cid, sig)) = buffer::with_buffer(buf_id, |b| {
                            Ok((b.change_id, buffer::content_signature(&b.lines)))
                        }) {
                            doc.saved_change_id = cid;
                            doc.saved_signature = sig;
                        }
                    }
                    redraw = true;
                    break;
                }
            }

            // Sidebar watcher: refresh when files are created/deleted/renamed.
            if subsystems.has_sidebar()
                && !project_root.is_empty()
                && sidebar_watcher.poll_changed()
            {
                let expanded: HashSet<String> = sidebar_entries
                    .iter()
                    .filter(|e| e.is_dir && e.expanded)
                    .map(|e| e.path.clone())
                    .collect();
                sidebar_entries = scan_for_sidebar(
                    subsystems.has_notes_mode(),
                    &project_root,
                    sidebar_show_hidden,
                );
                expand_sidebar_from_set(&mut sidebar_entries, &expanded, sidebar_show_hidden);
                sidebar_watcher.unwatch_all();
                sidebar_watcher.watch_dir(&project_root);
                for entry in &sidebar_entries {
                    if entry.is_dir && entry.expanded {
                        sidebar_watcher.watch_dir(&entry.path);
                    }
                }
                redraw = true;
            }

            // Notes-mode autosave: any dirty doc that has been idle (no
            // edit) for at least the debounce window gets persisted.
            // Keeps writes off the per-keystroke path while still
            // flushing within ~250 ms of typing pause.
            if subsystems.has_notes_mode() {
                let idle_threshold_secs = 0.25;
                let now = buffer::now_secs();
                for doc in docs.iter_mut() {
                    if doc.path.is_empty() {
                        continue;
                    }
                    let Some(buf_id) = doc.view.buffer_id else {
                        continue;
                    };
                    let needs_save = buffer::with_buffer(buf_id, |b| {
                        let dirty = b.change_id != doc.saved_change_id;
                        let idle = b
                            .last_edit
                            .map(|le| now - le.0 >= idle_threshold_secs)
                            .unwrap_or(true);
                        Ok(dirty && idle)
                    })
                    .unwrap_or(false);
                    if !needs_save {
                        continue;
                    }
                    let path = doc.path.clone();
                    let saved = buffer::with_buffer_mut(buf_id, |b| {
                        let crlf = b.crlf;
                        buffer::save_file(b, &path, crlf, false)
                            .map_err(|_| buffer::BufferError::UnknownBuffer)?;
                        Ok((b.change_id, buffer::content_signature(&b.lines)))
                    });
                    if let Ok((cid, sig)) = saved {
                        doc.saved_change_id = cid;
                        doc.saved_signature = sig;
                    }
                }
            }

            // Apply deferred render cache unconditionally so it never goes
            // stale. This MUST be outside the `if redraw` block -- otherwise
            // the cache sits unconsumed until the next event and forces an
            // infinite render loop if we try to force redraw when pending.
            if let Some((tab_idx, rendered_buf_id, lines, cid, sy, hint_count, rw, rh)) =
                pending_render_cache.take()
            {
                if let Some(doc_mut) = docs.get_mut(tab_idx) {
                    // Only apply the cache if the doc at this tab still wraps the
                    // same buffer that produced the render. A project switch
                    // (Open Recent) swaps the entire docs list, so tab_idx can
                    // alias a completely different file — in that case, a stale
                    // render would overwrite the fresh doc's empty cache and
                    // cause the previous project's text to flash on-screen.
                    if doc_mut.view.buffer_id == Some(rendered_buf_id) {
                        doc_mut.cached_render = lines;
                        doc_mut.cached_change_id = cid;
                        doc_mut.cached_scroll_y = sy;
                        doc_mut.cached_hint_count = hint_count;
                        doc_mut.cached_rect_w = rw;
                        doc_mut.cached_rect_h = rh;
                    }
                }
            }

            if redraw && window_hidden {
                // Consume the redraw flag but skip the actual render pass.
                // The compositor would throw away our frames anyway while
                // the window is occluded/minimised, and we've dropped the
                // glyph cache / render-cache buffers in the event handler.
                redraw = false;
            }
            if redraw {
                // Update window title and status bar from active tab.
                let app_name = "JereIDE";
                let active_doc_for_title = docs.get(active_tab);
                let title = active_doc_for_title
                    .map(|d| d.name.as_str())
                    .unwrap_or(app_name);
                let title_dirty = active_doc_for_title.is_some_and(doc_is_modified);
                let title_key = if title_dirty {
                    format!("*{title}")
                } else {
                    title.to_string()
                };
                if window_title != title_key {
                    let display =
                        crate::editor::doc_view::format_window_title(title, app_name, title_dirty);
                    crate::window::set_window_title(&display);
                    window_title = title_key;
                }
                status_view.left_items.clear();
                status_view.right_items.clear();
                if let Some(doc) = docs.get(active_tab) {
                    // Left: filename (with modified indicator). Cap at a
                    // third of the window so a runaway filename can't
                    // collide with the cursor-position segment or the
                    // right-side status items.
                    let modified_label = if doc_is_modified(doc) {
                        format!("*{}", doc.name)
                    } else {
                        doc.name.clone()
                    };
                    let filename_max_w = (width / 3.0).max(80.0);
                    let filename_display = {
                        use crate::editor::view::DrawContext as _;
                        if draw_ctx.font_width(style.font, &modified_label) <= filename_max_w {
                            modified_label
                        } else {
                            truncate_left_to_width(
                                &modified_label,
                                filename_max_w,
                                style.font,
                                &mut draw_ctx,
                            )
                        }
                    };
                    status_view.left_items.push(StatusItem {
                        text: filename_display,
                        color: None,
                        command: None,
                    });
                    // Left: cursor position + document %.
                    if let Some(buf_id) = doc.view.buffer_id {
                        let (line, col, total) = buffer::with_buffer(buf_id, |b| {
                            Ok((
                                *b.selections.get(2).unwrap_or(&1),
                                *b.selections.get(3).unwrap_or(&1),
                                b.lines.len(),
                            ))
                        })
                        .unwrap_or((1, 1, 1));
                        let pct = (line * 100).checked_div(total).unwrap_or(100);
                        status_view.left_items.push(StatusItem {
                            text: format!("  Ln {line}/{total}, Col {col}  ({pct}%)"),
                            color: Some(style.dim.to_array()),
                            command: None,
                        });
                    }
                    // Right side with separators: Lang | UTF-8 | Spaces: N | LF | INS
                    let ext = doc.path.rsplit('.').next().unwrap_or("");
                    let filename_for_lang =
                        doc.path.rsplit('/').next().unwrap_or(doc.path.as_str());
                    if status_lang_cache.0 != doc.path {
                        let name = crate::editor::syntax::match_syntax_entry(
                            filename_for_lang,
                            &syntax_index,
                        )
                        .map(|e| e.name.clone())
                        .unwrap_or_else(|| {
                            if ext.is_empty() {
                                "Plain Text".to_string()
                            } else {
                                ext.to_string()
                            }
                        });
                        status_lang_cache = (doc.path.clone(), name);
                    }
                    let lang: &str = &status_lang_cache.1;
                    let indent_label = if doc.indent_type == "hard" {
                        "Tabs".to_string()
                    } else {
                        format!("Spaces: {}", doc.indent_size)
                    };
                    let (crlf, huge) = doc
                        .view
                        .buffer_id
                        .and_then(|id| buffer::with_buffer(id, |b| Ok((b.crlf, b.is_huge()))).ok())
                        .unwrap_or((false, false));
                    let le = if crlf { "CRLF" } else { "LF" };
                    let mode = if overwrite_mode { "OVR" } else { "INS" };
                    let sep = " | ";
                    let mut right_parts = vec![
                        lang.to_string(),
                        "UTF-8".to_string(),
                        indent_label,
                        le.to_string(),
                    ];
                    if huge {
                        right_parts.push("No Undo".to_string());
                    }
                    if doc_is_modified(doc) {
                        right_parts.push("modified".to_string());
                    }
                    right_parts.push(mode.to_string());
                    let right_text = right_parts.join(sep);
                    status_view.right_items.push(StatusItem {
                        text: right_text,
                        color: Some(style.dim.to_array()),
                        command: None,
                    });
                } else {
                    status_view.left_items.push(StatusItem {
                        text: "JereIDE".to_string(),
                        color: None,
                        command: None,
                    });
                    status_view.right_items.push(StatusItem {
                        text: format!("v{}", env!("CARGO_PKG_VERSION")),
                        color: None,
                        command: None,
                    });
                }

                // Append LSP diagnostic count to status bar.
                if let Some(doc) = docs.get(active_tab) {
                    if let Some(diags) = lsp_state.diagnostics.get(&doc.path) {
                        if !diags.is_empty() {
                            let errors = diags.iter().filter(|d| d.severity == 1).count();
                            let warnings = diags.iter().filter(|d| d.severity == 2).count();
                            let label = if errors > 0 && warnings > 0 {
                                format!("{errors}E {warnings}W")
                            } else if errors > 0 {
                                format!("{errors}E")
                            } else {
                                format!("{warnings}W")
                            };
                            let color = if errors > 0 {
                                Some(style.error.to_array())
                            } else {
                                Some(style.warn.to_array())
                            };
                            status_view.right_items.insert(
                                0,
                                StatusItem {
                                    text: label,
                                    color,
                                    command: None,
                                },
                            );
                        }
                    }
                }

                // Momentum scrolling: velocity-driven with friction for wheel
                // events. Programmatic jumps (cursor, find, go-to) still set
                // `target_scroll_y` directly — when velocity is idle (< 0.5)
                // we snap to it instantly. Disabled via `disabled_transitions.scroll`.
                let dt = last_draw.elapsed().as_secs_f64().min(0.1);
                if config.transitions && !config.disabled_transitions.scroll && dt > 0.0 {
                    // --- Editor ---
                    if let Some(doc) = docs.get_mut(active_tab) {
                        let dv = &mut doc.view;
                        // Max scroll with 1.5 lines of overscroll past end.
                        let editor_max_scroll = dv
                            .buffer_id
                            .and_then(|id| {
                                let line_count =
                                    buffer::with_buffer(id, |b| Ok(b.lines.len())).ok()?;
                                let line_h = style.line_height();
                                let view_h = dv.rect().h;
                                Some(
                                    ((line_count as f64 * line_h) - view_h + line_h * 1.5).max(0.0),
                                )
                            })
                            .unwrap_or(0.0);
                        if editor_scroll_vel.abs() > 0.5 {
                            dv.scroll_y += editor_scroll_vel * dt;
                            dv.scroll_y = dv.scroll_y.clamp(0.0, editor_max_scroll);
                            dv.target_scroll_y = dv.scroll_y;
                            // Friction: exponential decay.
                            editor_scroll_vel *= (-30.0 * dt).exp();
                        } else {
                            editor_scroll_vel = 0.0;
                            // Snap programmatic jumps.
                            if dv.scroll_y != dv.target_scroll_y {
                                dv.scroll_y = dv.target_scroll_y;
                            }
                        }
                    }
                    // --- Sidebar ---
                    if subsystems.has_sidebar() && sidebar_visible {
                        if sidebar_scroll_vel.abs() > 0.5 {
                            sidebar_scroll += sidebar_scroll_vel * dt;
                            let max_scroll = (sidebar_content_h - sidebar_sb_h).max(0.0);
                            sidebar_scroll = sidebar_scroll.clamp(0.0, max_scroll);
                            sidebar_scroll_vel *= (-20.0 * dt).exp();
                        } else {
                            sidebar_scroll_vel = 0.0;
                        }
                    }
                    // --- Markdown preview ---
                    if let Some(doc) = docs.get_mut(active_tab) {
                        if doc.preview.enabled && preview_scroll_vel.abs() > 0.5 {
                            let rect = doc.preview.rect;
                            let line_h_pr = style.line_height();
                            let max_scroll =
                                (doc.preview.content_height - rect.h + line_h_pr * 1.5).max(0.0);
                            doc.preview.scroll_y += preview_scroll_vel * dt;
                            doc.preview.scroll_y = doc.preview.scroll_y.clamp(0.0, max_scroll);
                            doc.preview.target_scroll_y = doc.preview.scroll_y;
                            preview_scroll_vel *= (-20.0 * dt).exp();
                        } else {
                            preview_scroll_vel = 0.0;
                        }
                    }
                } else {
                    // Instant snap when transitions are disabled.
                    editor_scroll_vel = 0.0;
                    sidebar_scroll_vel = 0.0;
                    preview_scroll_vel = 0.0;
                    if let Some(doc) = docs.get_mut(active_tab) {
                        let dv = &mut doc.view;
                        if dv.scroll_y != dv.target_scroll_y {
                            dv.scroll_y = dv.target_scroll_y;
                        }
                    }
                    if subsystems.has_sidebar() && sidebar_visible {
                        let max_scroll = (sidebar_content_h - sidebar_sb_h).max(0.0);
                        sidebar_scroll = sidebar_scroll.clamp(0.0, max_scroll);
                    }
                }

                crate::renderer::native_begin_frame();
                crate::editor::app_state::clip_init(width, height);

                // Tab-bar overlay state captured during the tab draw pass and
                // consumed later (just before native_end_frame) to render the
                // hover tooltip and overflow dropdown list. Drawing those at
                // the end keeps them on top of the sidebar / breadcrumb / doc
                // view — otherwise the breadcrumb would paint over them.
                let mut tab_hover: Option<usize> = None;
                let mut tab_overlay_tbh: f64 = 0.0;
                let mut tab_overlay_overflow: bool = false;
                let mut tab_overlay_rects: Vec<(f64, f64, String, String)> = Vec::new();
                let mut tab_overlay_btn_right: f64 = width;
                let mut tab_overlay_btn_w: f64 = 0.0;

                // Draw tab bar (hidden in single-file mode).
                let _tab_bar_h = if !single_file_mode && !docs.is_empty() {
                    let tbh = style.font_height + style.padding_y * 3.0;
                    let accent_h = 3.0;
                    use crate::editor::view::DrawContext as _;
                    draw_ctx.draw_rect(
                        sidebar_w,
                        0.0,
                        width - sidebar_w,
                        tbh,
                        style.background2.to_array(),
                    );

                    let close_w = draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                    let dropdown_btn_w = (style.font_height + style.padding_x * 2.0).ceil();

                    // Measure full-width tab bar (no truncation) to decide whether to
                    // enter overflow mode. Reserving the dropdown button space keeps
                    // the decision stable once overflow is on.
                    let avail_full = (width - sidebar_w).max(0.0);
                    let mut full_total = 0.0_f64;
                    for doc in docs.iter() {
                        let label = if doc_is_modified(doc) {
                            format!("*{}", doc.name)
                        } else {
                            doc.name.clone()
                        };
                        full_total += draw_ctx.font_width(style.font, &label)
                            + style.padding_x * 2.0
                            + close_w
                            + style.divider_size;
                    }
                    let tabs_overflow = full_total > avail_full;
                    if !tabs_overflow {
                        tab_dropdown_open = false;
                    }
                    let tabs_right_limit = if tabs_overflow {
                        (width - dropdown_btn_w).max(sidebar_w)
                    } else {
                        width
                    };
                    tab_overlay_tbh = tbh;
                    tab_overlay_overflow = tabs_overflow;
                    tab_overlay_btn_right = width;
                    tab_overlay_btn_w = dropdown_btn_w;

                    // Cache displayed labels (with truncation when overflowing) and
                    // per-tab rects so the tooltip pass below and the hit-tests can
                    // reuse them without recomputing widths.
                    let mut tab_rects: Vec<(f64, f64, String, String)> =
                        Vec::with_capacity(docs.len());

                    let mut tx = sidebar_w;
                    for (i, doc) in docs.iter().enumerate() {
                        let full_label = if doc_is_modified(doc) {
                            format!("*{}", doc.name)
                        } else {
                            doc.name.clone()
                        };
                        let display_label = if tabs_overflow {
                            let base = truncate_tab_name(&doc.name, 10);
                            if doc_is_modified(doc) {
                                format!("*{base}")
                            } else {
                                base
                            }
                        } else {
                            full_label.clone()
                        };
                        let tw = draw_ctx.font_width(style.font, &display_label)
                            + style.padding_x * 2.0
                            + close_w;
                        tab_rects.push((tx, tw, display_label.clone(), full_label.clone()));
                        // Don't draw tabs that fall entirely past the dropdown limit;
                        // they're still reachable via the dropdown menu.
                        if tx >= tabs_right_limit {
                            tx += tw + style.divider_size;
                            continue;
                        }
                        let bg = if i == active_tab {
                            style.background.to_array()
                        } else {
                            style.background2.to_array()
                        };
                        let fg = if i == active_tab {
                            style.text.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        // Clip this tab to the area left of the dropdown button.
                        let tab_visible_w = (tabs_right_limit - tx).max(0.0).min(tw);
                        draw_ctx.set_clip_rect(tx, 0.0, tab_visible_w, tbh);
                        draw_ctx.draw_rect(tx, accent_h, tw, tbh - accent_h, bg);
                        if i == active_tab {
                            draw_ctx.draw_rect(tx, 0.0, tw, accent_h, style.accent.to_array());
                        }
                        let text_y_tab = accent_h + (tbh - accent_h - style.font_height) / 2.0;
                        draw_ctx.draw_text(
                            style.font,
                            &display_label,
                            tx + style.padding_x,
                            text_y_tab,
                            fg,
                        );
                        // Close button with hover highlight.
                        let close_x = tx + tw - close_w;
                        let close_hovered =
                            mouse_y < tbh && mouse_x >= close_x && mouse_x < close_x + close_w;
                        if close_hovered {
                            draw_ctx.draw_rect(
                                close_x,
                                accent_h,
                                close_w,
                                tbh - accent_h,
                                style.line_highlight.to_array(),
                            );
                        }
                        let close_color = if close_hovered {
                            style.text.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        draw_ctx.draw_text(
                            style.icon_font,
                            "C",
                            close_x + style.padding_x * 0.5,
                            accent_h
                                + (tbh - accent_h - draw_ctx.font_height(style.icon_font)) / 2.0,
                            close_color,
                        );
                        draw_ctx.draw_rect(
                            tx + tw,
                            style.padding_y * 0.5,
                            style.divider_size,
                            tbh - style.padding_y,
                            style.dim.to_array(),
                        );
                        // Restore clip for the rest of the tab bar / dropdown draw.
                        crate::editor::app_state::clip_init(width, height);

                        // Track hover for tooltip: only when not over the close icon,
                        // so the close-button interaction is unambiguous.
                        if mouse_y < tbh
                            && mouse_x >= tx
                            && mouse_x < (tx + tw).min(tabs_right_limit)
                            && !close_hovered
                        {
                            tab_hover = Some(i);
                        }
                        tx += tw + style.divider_size;
                    }
                    if mouse_y >= tbh {
                        tab_tooltip_suppressed = false;
                    }

                    // Overflow dropdown button. The arrow is drawn as a filled
                    // triangle built from horizontal one-pixel bars rather than a
                    // font glyph — the icons.ttf bundle doesn't include a
                    // chevron-down, and the regular font's "v" looked like a
                    // letter, not an icon.
                    if tabs_overflow {
                        let btn_x = width - dropdown_btn_w;
                        let btn_hovered = mouse_y < tbh && mouse_x >= btn_x;
                        let btn_bg = if btn_hovered || tab_dropdown_open {
                            style.line_highlight.to_array()
                        } else {
                            style.background2.to_array()
                        };
                        draw_ctx.draw_rect(btn_x, accent_h, dropdown_btn_w, tbh - accent_h, btn_bg);
                        draw_ctx.draw_rect(
                            btn_x,
                            accent_h,
                            style.divider_size,
                            tbh - accent_h,
                            style.divider.to_array(),
                        );
                        let arrow_color = style.text.to_array();
                        let arrow_h = (style.font_height * 0.45).round().max(4.0);
                        let arrow_w_px = arrow_h * 2.0;
                        let arrow_cx = btn_x + dropdown_btn_w / 2.0;
                        let arrow_top = accent_h + (tbh - accent_h - arrow_h) / 2.0;
                        let rows = arrow_h as i32;
                        for i in 0..rows {
                            let progress = i as f64 / rows as f64;
                            let row_w = (arrow_w_px * (1.0 - progress)).max(1.0);
                            let row_x = arrow_cx - row_w / 2.0;
                            let row_y = arrow_top + i as f64;
                            draw_ctx.draw_rect(row_x, row_y, row_w, 1.0, arrow_color);
                        }
                    } else {
                        tab_dropdown_open = false;
                    }

                    draw_ctx.draw_rect(
                        sidebar_w,
                        tbh - style.divider_size,
                        width - sidebar_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    crate::editor::app_state::clip_init(width, height);

                    // Hand off per-tab rects to the deferred overlay pass. That
                    // pass runs after every other panel has drawn, so the tooltip
                    // and overflow dropdown aren't painted over by the breadcrumb
                    // / sidebar / doc view that follow this block.
                    tab_overlay_rects = tab_rects;

                    tbh
                } else {
                    tab_dropdown_open = false;
                    0.0
                };

                // Draw breadcrumb strip above the document area.
                if let Some(doc) = docs.get(active_tab) {
                    crate::editor::doc_view::draw_breadcrumb(
                        &mut draw_ctx,
                        &doc.path,
                        sidebar_w,
                        tab_h,
                        width - sidebar_w - minimap_w,
                        breadcrumb_h,
                        &style,
                    );
                }

                // Draw sidebar.
                if subsystems.has_sidebar() && sidebar_visible {
                    use crate::editor::view::DrawContext as _;
                    draw_ctx.draw_rect(0.0, 0.0, sidebar_w, height, style.background2.to_array());

                    // Mini toolbar at the top of the sidebar (big icon font).
                    // When the toolbar subsystem is off (Note-Anvil), collapse
                    // the reserved height so the directory header sits flush
                    // with the top instead of leaving an empty strip.
                    let ibf = style.icon_big_font;
                    let icon_h = draw_ctx.font_height(ibf);
                    let toolbar_h = if subsystems.has_toolbar() {
                        icon_h + style.padding_y * 2.0
                    } else {
                        0.0
                    };
                    if subsystems.has_toolbar() {
                        draw_ctx.draw_rect(
                            0.0,
                            0.0,
                            sidebar_w,
                            toolbar_h,
                            style.background3.to_array(),
                        );
                        let toolbar_buttons: &[(&str, &str)] = &[
                            ("f", "core:new-doc"),
                            ("D", "core:open-file"),
                            ("S", "doc:save"),
                            ("L", "find-replace:find"),
                            ("B", "core:find-command"),
                            ("P", "core:open-user-settings"),
                        ];
                        let mut bx = style.padding_x;
                        let btn_y = (toolbar_h - icon_h) / 2.0;
                        let icon_spacing = style.padding_x;
                        for (icon, _cmd) in toolbar_buttons {
                            let iw = draw_ctx.font_width(ibf, icon);
                            if bx + iw + icon_spacing > sidebar_w {
                                break;
                            }
                            draw_ctx.draw_text(ibf, icon, bx, btn_y, style.dim.to_array());
                            bx += iw + icon_spacing;
                        }
                        draw_ctx.draw_rect(
                            0.0,
                            toolbar_h - style.divider_size,
                            sidebar_w,
                            style.divider_size,
                            style.divider.to_array(),
                        );
                    }

                    // Project directory name header.
                    let dir_header_h = style.font_height + style.padding_y;
                    let resolved_root = std::fs::canonicalize(&project_root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| project_root.clone());
                    let dir_name = resolved_root
                        .rsplit('/')
                        .find(|s| !s.is_empty())
                        .unwrap_or(&resolved_root);
                    // Ellipsize if the folder name overflows the sidebar width.
                    let header_avail =
                        (sidebar_w - style.padding_x * 2.0 - style.divider_size).max(0.0);
                    let dir_label: String = if draw_ctx.font_width(style.font, dir_name)
                        <= header_avail
                    {
                        dir_name.to_string()
                    } else {
                        let ell = "...";
                        let ell_w = draw_ctx.font_width(style.font, ell);
                        let chars: Vec<char> = dir_name.chars().collect();
                        let mut fit = String::new();
                        for take in (0..chars.len()).rev() {
                            let candidate: String = chars[..take].iter().collect();
                            if draw_ctx.font_width(style.font, &candidate) + ell_w <= header_avail {
                                fit = format!("{candidate}{ell}");
                                break;
                            }
                        }
                        if fit.is_empty() { ell.to_string() } else { fit }
                    };
                    draw_ctx.draw_rect(
                        0.0,
                        toolbar_h,
                        sidebar_w,
                        dir_header_h,
                        style.background2.to_array(),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &dir_label,
                        style.padding_x,
                        toolbar_h + (dir_header_h - style.font_height) / 2.0,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_rect(
                        0.0,
                        toolbar_h + dir_header_h - style.divider_size,
                        sidebar_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Notes-mode: sort toggle + search box between the
                    // directory header and the file list.
                    let notes_row_h = if subsystems.has_notes_mode() {
                        let row_h = style.font_height + style.padding_y * 2.0;
                        let search_h = style.font_height + style.padding_y * 2.0;
                        let bar_y = toolbar_h + dir_header_h;
                        // Sort-toggle row background.
                        draw_ctx.draw_rect(
                            0.0,
                            bar_y,
                            sidebar_w,
                            row_h,
                            style.background2.to_array(),
                        );
                        let half = (sidebar_w / 2.0).floor();
                        let is_alpha = notes_sort_mode <= 1;
                        let is_recent = notes_sort_mode >= 2;
                        let arrow = |asc: bool| if asc { "\u{2191}" } else { "\u{2193}" };
                        let alpha_arrow = arrow(notes_sort_mode == 0);
                        let recent_arrow = arrow(notes_sort_mode == 3);
                        let alpha_label = format!("A-Z {alpha_arrow}");
                        let recent_label = format!("Recent {recent_arrow}");
                        if is_alpha {
                            draw_ctx.draw_rect(
                                0.0,
                                bar_y,
                                half,
                                row_h,
                                style.line_highlight.to_array(),
                            );
                        }
                        if is_recent {
                            draw_ctx.draw_rect(
                                half,
                                bar_y,
                                sidebar_w - half,
                                row_h,
                                style.line_highlight.to_array(),
                            );
                        }
                        let alpha_w = draw_ctx.font_width(style.font, &alpha_label);
                        let recent_w = draw_ctx.font_width(style.font, &recent_label);
                        let text_y = bar_y + (row_h - style.font_height) / 2.0;
                        draw_ctx.draw_text(
                            style.font,
                            &alpha_label,
                            (half - alpha_w) / 2.0,
                            text_y,
                            if is_alpha {
                                style.accent.to_array()
                            } else {
                                style.dim.to_array()
                            },
                        );
                        draw_ctx.draw_text(
                            style.font,
                            &recent_label,
                            half + (sidebar_w - half - recent_w) / 2.0,
                            text_y,
                            if is_recent {
                                style.accent.to_array()
                            } else {
                                style.dim.to_array()
                            },
                        );
                        draw_ctx.draw_rect(
                            half,
                            bar_y + style.padding_y * 0.3,
                            style.divider_size,
                            row_h - style.padding_y * 0.6,
                            style.divider.to_array(),
                        );
                        // Search input row.
                        let search_y = bar_y + row_h;
                        let search_bg = if notes_search_focused {
                            style.background.to_array()
                        } else {
                            style.background3.to_array()
                        };
                        draw_ctx.draw_rect(
                            style.padding_x,
                            search_y + style.padding_y * 0.4,
                            sidebar_w - style.padding_x * 2.0,
                            search_h - style.padding_y * 0.8,
                            search_bg,
                        );
                        let label = if notes_search.is_empty() && !notes_search_focused {
                            "Search notes..."
                        } else {
                            notes_search.as_str()
                        };
                        let label_color = if notes_search.is_empty() && !notes_search_focused {
                            style.dim.to_array()
                        } else {
                            style.text.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            label,
                            style.padding_x * 2.0,
                            search_y + (search_h - style.font_height) / 2.0,
                            label_color,
                        );
                        // Caret when focused.
                        if notes_search_focused {
                            let caret_x = style.padding_x * 2.0
                                + draw_ctx.font_width(style.font, &notes_search);
                            draw_ctx.draw_rect(
                                caret_x,
                                search_y + style.padding_y * 0.5,
                                1.0,
                                style.font_height,
                                style.caret.to_array(),
                            );
                        }
                        draw_ctx.draw_rect(
                            0.0,
                            bar_y + row_h + search_h - style.divider_size,
                            sidebar_w,
                            style.divider_size,
                            style.divider.to_array(),
                        );
                        row_h + search_h
                    } else {
                        0.0
                    };

                    // File tree entries — clip to the area below the header so
                    // scrolled entries don't overdraw the toolbar or folder name.
                    let entry_h = style.font_height + style.padding_y;
                    let icon_font_h = draw_ctx.font_height(style.icon_font);
                    let icon_w = draw_ctx.font_width(style.icon_font, "D") + style.padding_x * 0.5;
                    let active_path = docs.get(active_tab).map(|d| d.path.as_str()).unwrap_or("");
                    let sidebar_content_top = toolbar_h + dir_header_h + notes_row_h;
                    draw_ctx.set_clip_rect(
                        0.0,
                        sidebar_content_top,
                        sidebar_w,
                        height - sidebar_content_top,
                    );
                    let notes_display: Vec<usize> = if subsystems.has_notes_mode() {
                        compute_notes_display_order(
                            &sidebar_entries,
                            &notes_search,
                            notes_sort_mode,
                        )
                    } else {
                        (0..sidebar_entries.len()).collect()
                    };
                    let mut ey = toolbar_h + dir_header_h + notes_row_h - sidebar_scroll;
                    sidebar_hovered_index = None;
                    if !context_menu.visible {
                        sidebar_menu_pinned_index = None;
                    }
                    for &disp_idx in &notes_display {
                        let entry = &sidebar_entries[disp_idx];
                        if ey + entry_h > sidebar_content_top && ey < height {
                            // Track which entry the mouse is over.
                            if mouse_x >= 0.0
                                && mouse_x < sidebar_w
                                && mouse_y >= ey
                                && mouse_y < ey + entry_h
                            {
                                sidebar_hovered_index = Some(disp_idx);
                            }
                            let is_highlighted = sidebar_hovered_index == Some(disp_idx)
                                || sidebar_menu_pinned_index == Some(disp_idx);
                            let indent = entry.depth as f64 * style.padding_x * 1.5;
                            let x = style.padding_x + indent;
                            let text_y = ey + (entry_h - style.font_height) / 2.0;

                            // Highlight active file.
                            let is_active = !entry.is_dir && entry.path == active_path;
                            if is_active {
                                let mut hl = style.line_highlight.to_array();
                                hl[3] = 210.min(hl[3].saturating_add(100));
                                draw_ctx.draw_rect(0.0, ey, sidebar_w, entry_h, hl);
                            }

                            // Icon (vertically centered in the row).
                            if entry.is_dir {
                                let icon = if entry.expanded { "D" } else { "d" };
                                let icon_y = ey + (entry_h - icon_font_h) / 2.0;
                                // Centre the folder glyph's advance in the
                                // icon column the same way file icons are
                                // centred — otherwise folder rows looked
                                // outdented next to the now-centred file
                                // rows.
                                let folder_w = draw_ctx.font_width(style.icon_font, icon);
                                let folder_x = x + (icon_w - folder_w) / 2.0;
                                draw_ctx.draw_text(
                                    style.icon_font,
                                    icon,
                                    folder_x,
                                    icon_y,
                                    if is_highlighted {
                                        style.accent.to_array()
                                    } else {
                                        style.text.to_array()
                                    },
                                );
                            } else {
                                // Seti file-type icon glyph.
                                let ext = entry.name.rsplit('.').next().unwrap_or("");
                                let icon_info = file_icons
                                    .get(ext)
                                    .or_else(|| file_icons.get(entry.name.as_str()))
                                    .or_else(|| file_icons.get("_default"));
                                if let Some(fi) = icon_info {
                                    let glyph = char::from_u32(fi.codepoint)
                                        .map(|c| c.to_string())
                                        .unwrap_or_default();
                                    // Codepoints below seti.ttf's private-use
                                    // range (U+E000+) aren't in that font; use
                                    // the body font so `file_icons.json` can
                                    // map an extension to a plain ASCII letter
                                    // (e.g. `G` for Gossamer). Body letters
                                    // render smaller than the surrounding
                                    // seti glyphs — the centring math below
                                    // still places them on-axis, just at the
                                    // body font's natural visual weight.
                                    let icon_font = if fi.codepoint < 0xE000 {
                                        style.font
                                    } else {
                                        style.seti_font
                                    };
                                    // Vertical: centre against seti's line
                                    // height regardless of which font drew it,
                                    // so a body-font letter sits on the same
                                    // baseline as the seti icons in adjacent
                                    // rows.
                                    let seti_h = draw_ctx.font_height(style.seti_font);
                                    let icon_y = ey + (entry_h - seti_h) / 2.0;
                                    // Horizontal: centre each glyph's advance
                                    // box in the icon column. The default
                                    // plaintext seti glyph has an advance
                                    // wider than `icon_w` and so produces a
                                    // negative offset — that's intentional, a
                                    // small leftward bleed into the indent
                                    // gutter is invisible and pulls the
                                    // glyph's visual centre back over the
                                    // column centre. Without it, plaintext
                                    // (and any other wide-advance icon) read
                                    // as lopsided to the right.
                                    let glyph_w = draw_ctx.font_width(icon_font, &glyph);
                                    let icon_x = x + (icon_w - glyph_w) / 2.0;
                                    draw_ctx.draw_text(icon_font, &glyph, icon_x, icon_y, fi.color);
                                }
                            }

                            // Name (vertically centered, same baseline alignment).
                            // Add spacing between icon and name.
                            let name_x = x + icon_w + style.padding_x * 0.7;
                            let name_color = if is_highlighted {
                                style.accent.to_array()
                            } else {
                                style.text.to_array()
                            };
                            // Ellipsize if the name would overflow the sidebar width.
                            let avail = (sidebar_w - name_x - style.padding_x - style.divider_size)
                                .max(0.0);
                            let display_name: String =
                                if draw_ctx.font_width(style.font, &entry.name) <= avail {
                                    entry.name.clone()
                                } else {
                                    let ell = "...";
                                    let ell_w = draw_ctx.font_width(style.font, ell);
                                    let chars: Vec<char> = entry.name.chars().collect();
                                    let mut fit = String::new();
                                    for take in (0..chars.len()).rev() {
                                        let candidate: String = chars[..take].iter().collect();
                                        if draw_ctx.font_width(style.font, &candidate) + ell_w
                                            <= avail
                                        {
                                            fit = format!("{candidate}{ell}");
                                            break;
                                        }
                                    }
                                    if fit.is_empty() { ell.to_string() } else { fit }
                                };
                            draw_ctx.draw_text(
                                style.font,
                                &display_name,
                                name_x,
                                text_y,
                                name_color,
                            );
                        }
                        ey += entry_h;
                    }
                    // Inline new-file input: draws an extra row at the bottom
                    // of the target directory's children.
                    if let Some(ref new_dir) = sidebar_new_file_dir {
                        // Find the display row to insert after (the last entry
                        // still inside `new_dir`, or right after the dir itself).
                        let mut insert_disp_row = notes_display.len();
                        let mut nf_dir_depth = 0usize;
                        let mut found_dir = false;
                        for (row, &disp_idx) in notes_display.iter().enumerate() {
                            let e = &sidebar_entries[disp_idx];
                            if !found_dir {
                                if e.is_dir && &e.path == new_dir {
                                    found_dir = true;
                                    nf_dir_depth = e.depth;
                                }
                            } else if e.depth <= nf_dir_depth {
                                insert_disp_row = row;
                                break;
                            }
                        }
                        let nf_indent = (nf_dir_depth + 1) as f64 * style.padding_x * 1.5;
                        let nf_x = style.padding_x + nf_indent;
                        let nf_y = toolbar_h + dir_header_h + notes_row_h - sidebar_scroll
                            + insert_disp_row as f64 * entry_h;
                        if nf_y + entry_h > sidebar_content_top && nf_y < height {
                            // Selection-tinted row background.
                            draw_ctx.draw_rect(
                                0.0,
                                nf_y,
                                sidebar_w,
                                entry_h,
                                style.selection.to_array(),
                            );
                            // Text and cursor for the filename being typed.
                            let text_x = nf_x + icon_w + style.padding_x * 0.7;
                            let text_y_pos = nf_y + (entry_h - style.font_height) / 2.0;
                            draw_ctx.draw_text(
                                style.font,
                                &sidebar_new_file_name,
                                text_x,
                                text_y_pos,
                                style.text.to_array(),
                            );
                            let cursor_safe =
                                sidebar_new_file_cursor.min(sidebar_new_file_name.len());
                            let before_cursor = &sidebar_new_file_name[..cursor_safe];
                            let cursor_x = text_x + draw_ctx.font_width(style.font, before_cursor);
                            draw_ctx.draw_rect(
                                cursor_x,
                                text_y_pos,
                                style.caret_width,
                                style.font_height,
                                style.caret.to_array(),
                            );
                        }
                    }

                    // Reset clip to full window for the sidebar edge divider.
                    crate::editor::app_state::clip_init(width, height);

                    // Sidebar scrollbar (lite-xl style): proportional thumb
                    // with a minimum size, drawn just inside the right edge.
                    let extra_row = sidebar_new_file_dir.is_some() as usize;
                    let total_entries_h = (notes_display.len() + extra_row) as f64 * entry_h;
                    let sb_area_y = sidebar_content_top;
                    let sb_area_h = (height - sidebar_content_top).max(0.0);
                    sidebar_content_h = total_entries_h;
                    sidebar_sb_top = sb_area_y;
                    sidebar_sb_h = sb_area_h;
                    if total_entries_h > sb_area_h && sb_area_h > 0.0 {
                        let sb_w = style.scrollbar_size;
                        let sb_x = sidebar_w - style.divider_size - sb_w;
                        draw_ctx.draw_rect(
                            sb_x,
                            sb_area_y,
                            sb_w,
                            sb_area_h,
                            style.scrollbar_track.to_array(),
                        );
                        let ratio = sb_area_h / total_entries_h;
                        let min_thumb = style.scrollbar_size * 2.0;
                        let thumb_h = (sb_area_h * ratio).max(min_thumb).min(sb_area_h);
                        let max_scroll = (total_entries_h - sb_area_h).max(1.0);
                        let scroll_frac = (sidebar_scroll / max_scroll).clamp(0.0, 1.0);
                        let thumb_y = sb_area_y + scroll_frac * (sb_area_h - thumb_h);
                        draw_ctx.draw_rect(
                            sb_x,
                            thumb_y,
                            sb_w,
                            thumb_h,
                            style.scrollbar.to_array(),
                        );
                    }

                    // Divider on the right edge.
                    draw_ctx.draw_rect(
                        sidebar_w - style.divider_size,
                        0.0,
                        style.divider_size,
                        height,
                        style.divider.to_array(),
                    );
                    crate::editor::app_state::clip_init(width, height);
                }

                if let Some(doc) = docs.get(active_tab) {
                    let dv = &doc.view;
                    if let Some(buf_id) = dv.buffer_id {
                        let ext = doc.path.rsplit('.').next().unwrap_or("");
                        // Compile-on-demand and bump MRU. Evict the LRU
                        // entry once the cache exceeds SYNTAX_CACHE_CAP
                        // so memory doesn't grow unbounded on sessions
                        // that touch many file types.
                        let ext_owned = ext.to_string();
                        compiled_syntax_mru.retain(|e| e != &ext_owned);
                        compiled_syntax_mru.insert(0, ext_owned.clone());
                        while compiled_syntax_mru.len() > SYNTAX_CACHE_CAP {
                            if let Some(drop_ext) = compiled_syntax_mru.pop() {
                                compiled_syntax_cache.remove(&drop_ext);
                            }
                        }
                        let compiled_opt =
                            compiled_syntax_cache.entry(ext_owned).or_insert_with(|| {
                                let filename = doc.path.rsplit('/').next().unwrap_or(&doc.path);
                                let entry = crate::editor::syntax::match_syntax_entry(
                                    filename,
                                    &syntax_index,
                                )?;
                                let def = entry.load_full()?;
                                match tokenizer::compile_from_definition(&def) {
                                    Ok(cs) => Some(cs),
                                    Err(e) => {
                                        log_to_file(
                                            userdir,
                                            &format!("Syntax compile error: {e:?}"),
                                        );
                                        None
                                    }
                                }
                            });
                        let wrap_w = if line_wrapping {
                            Some(dv.rect().w - dv.gutter_width - style.padding_x * 2.0)
                        } else {
                            None
                        };
                        let is_lsp_file = ext_to_lsp_filetype(ext)
                            .map(|ft| ft == lsp_state.filetype)
                            .unwrap_or(false);
                        let active_uri = if doc.path.is_empty() {
                            String::new()
                        } else {
                            path_to_uri(&doc.path)
                        };
                        let empty_hints = Vec::new();
                        // Only use held hints if they belong to the active file.
                        // After a tab-switch the cached `inlay_hints` still
                        // contain entries from the previous file; rendering
                        // them here would show ghost hints at mismatched line
                        // numbers until the new file's response arrives.
                        let hints = if subsystems.has_lsp()
                            && is_lsp_file
                            && lsp_state.inlay_hints_uri == active_uri
                        {
                            &lsp_state.inlay_hints
                        } else {
                            &empty_hints
                        };
                        // Cache render lines to avoid re-tokenizing on every
                        // cursor move. Invalidate when hint count changes so LSP
                        // inlay hints appear as soon as they arrive.
                        let current_change_id =
                            buffer::with_buffer(buf_id, |b| Ok(b.change_id)).unwrap_or(0);
                        let scroll_y_now = dv.scroll_y;
                        let hint_count_now = hints.len();
                        // `cached_render` is Arc-shared so the cache-hit
                        // path is a refcount bump rather than a full
                        // `Vec<RenderLine>` clone per redraw.
                        let render_lines: std::sync::Arc<Vec<RenderLine>> =
                            if let Some(doc) = docs.get(active_tab) {
                                if doc.cached_change_id == current_change_id
                                    && (doc.cached_scroll_y - scroll_y_now).abs() < 0.5
                                    && doc.cached_hint_count == hint_count_now
                                    && (doc.cached_rect_w - dv.rect().w).abs() < 0.5
                                    && (doc.cached_rect_h - dv.rect().h).abs() < 0.5
                                    && !doc.cached_render.is_empty()
                                {
                                    std::sync::Arc::clone(&doc.cached_render)
                                } else {
                                    std::sync::Arc::new(build_render_lines(
                                        buf_id,
                                        dv,
                                        &style,
                                        ext,
                                        compiled_opt.as_ref(),
                                        wrap_w,
                                        hints,
                                        Some(&doc.token_cache),
                                    ))
                                }
                            } else {
                                std::sync::Arc::new(build_render_lines(
                                    buf_id,
                                    dv,
                                    &style,
                                    ext,
                                    compiled_opt.as_ref(),
                                    wrap_w,
                                    hints,
                                    Some(&doc.token_cache),
                                ))
                            };
                        let (sel, cursor_line, cursor_col, all_cursors) =
                            buffer::with_buffer(buf_id, |b| {
                                let mut sels = Vec::new();
                                let mut cursors = Vec::new();
                                let n = buffer::cursor_count(b);
                                for i in 0..n {
                                    let base = i * 4;
                                    let l1 = b.selections[base];
                                    let c1 = b.selections[base + 1];
                                    let l2 = b.selections[base + 2];
                                    let c2 = b.selections[base + 3];
                                    cursors.push((l2, c2));
                                    if l1 != l2 || c1 != c2 {
                                        let (sl1, sc1, sl2, sc2) =
                                            if l1 < l2 || (l1 == l2 && c1 <= c2) {
                                                (l1, c1, l2, c2)
                                            } else {
                                                (l2, c2, l1, c1)
                                            };
                                        sels.push(crate::editor::doc_view::SelectionRange {
                                            line1: sl1,
                                            col1: sc1,
                                            line2: sl2,
                                            col2: sc2,
                                        });
                                    }
                                }
                                // Primary cursor is the first one (for scrolling).
                                let pl = b.selections.get(2).copied().unwrap_or(1);
                                let pc = b.selections.get(3).copied().unwrap_or(1);
                                Ok((sels, pl, pc, cursors))
                            })
                            .unwrap_or((vec![], 1, 1, vec![(1, 1)]));
                        let elapsed_since_reset = cursor_blink_reset.elapsed().as_secs_f64();
                        let cursor_on = elapsed_since_reset < blink_period
                            || (elapsed_since_reset % (blink_period * 2.0)) < blink_period;
                        // Highlight other occurrences of a compact, single-line,
                        // whitespace-free selection (a "word").
                        let occurrence: String = doc
                            .view
                            .buffer_id
                            .and_then(|bid| {
                                buffer::with_buffer(bid, |b| {
                                    let s = &b.selections;
                                    if s.len() == 4 && s[0] == s[2] && s[1] != s[3] {
                                        let (cs, ce) = if s[1] < s[3] {
                                            (s[1], s[3])
                                        } else {
                                            (s[3], s[1])
                                        };
                                        let text = b
                                            .lines
                                            .get(s[0].saturating_sub(1))
                                            .map(|l| l.trim_end_matches('\n'))
                                            .unwrap_or("");
                                        let word: String = text
                                            .chars()
                                            .skip(cs.saturating_sub(1))
                                            .take(ce - cs)
                                            .collect();
                                        let ok = !word.is_empty()
                                            && word.chars().count() <= 100
                                            && word.chars().all(|ch| !ch.is_whitespace());
                                        Ok(if ok { word } else { String::new() })
                                    } else {
                                        Ok(String::new())
                                    }
                                })
                                .ok()
                            })
                            .unwrap_or_default();
                        let bracket = dv.buffer_id.and_then(|buf_id| {
                            buffer::with_buffer(buf_id, |b| {
                                Ok(crate::editor::picker::bracket_pair(
                                    &b.lines, cursor_line, cursor_col,
                                ))
                            })
                            .ok()
                            .flatten()
                        });
                        dv.draw_native(
                            &mut draw_ctx,
                            &style,
                            &render_lines,
                            &sel,
                            cursor_line,
                            cursor_col,
                            cursor_on,
                            &doc.git_changes,
                            &all_cursors,
                            &occurrence,
                            bracket,
                        );

                        // Test-runner badges: scan the doc for recognised
                        // test definitions and paint a "Run test" CodeLens-
                        // style hint in `style.dim` (greys with the theme,
                        // matches VS Code's descriptionForeground). Only
                        // runs if a runner can be detected -- no point
                        // offering the affordance if nothing can execute.
                        use crate::editor::view::DrawContext as _;
                        test_badges.clear();
                        if !doc.path.is_empty() {
                            // Rescan only when the file or its content changed;
                            // detection probes the filesystem and discovery
                            // clones the whole document, so neither can run on
                            // every redraw (scroll, cursor blink, mouse move).
                            if test_scan_cache.0 != doc.path
                                || test_scan_cache.1 != current_change_id
                            {
                                let has_runner =
                                    crate::editor::test_runner::detect_runner_with_fallback(
                                        &project_root,
                                        &doc.path,
                                    )
                                    .is_some();
                                active_tests = if has_runner {
                                    let text_lines = buffer::with_buffer(buf_id, |b| {
                                        Ok(b.lines
                                            .iter()
                                            .map(|l| l.trim_end_matches('\n').to_string())
                                            .collect::<Vec<_>>())
                                    })
                                    .unwrap_or_default();
                                    crate::editor::test_runner::discover_tests(
                                        &doc.path,
                                        &text_lines,
                                    )
                                } else {
                                    Vec::new()
                                };
                                test_scan_cache = (doc.path.clone(), current_change_id);
                            }
                            // Render loops over `active_tests`, which is empty
                            // when no runner was detected, so this is a no-op in
                            // that case without a second guard.
                            let line_h = style.line_height();
                            let dv_rect = dv.rect();
                            // Plain ASCII text so no font has to carry a
                            // triangle glyph; the previous "▶" rendered as
                            // a .notdef box in Lilex and other code fonts
                            // that don't cover U+25B6.
                            let badge_text = "Run test";
                            let badge_w =
                                draw_ctx.font_width(style.font, badge_text) + style.padding_x;
                            for (i, test) in active_tests.iter().enumerate() {
                                // Render on the SAME row as the `fn` line
                                // (`test.line`), right-aligned. That puts
                                // the hint visually below any decorator /
                                // #[test] attribute and above the function
                                // body -- the closest single-row
                                // approximation to VS Code's dedicated
                                // CodeLens row. Right-aligning keeps it
                                // away from the fn signature for most
                                // common fn widths.
                                let fn_line = test.line.max(1);
                                let row_y =
                                    dv_rect.y + (fn_line as f64 - 1.0) * line_h - dv.scroll_y;
                                if row_y + line_h < dv_rect.y || row_y >= dv_rect.y + dv_rect.h {
                                    continue;
                                }
                                let badge_x = (dv_rect.x + dv_rect.w
                                    - style.scrollbar_size
                                    - badge_w
                                    - style.padding_x)
                                    .max(dv_rect.x);
                                draw_ctx.draw_text(
                                    style.font,
                                    badge_text,
                                    badge_x,
                                    row_y + (line_h - style.font_height) / 2.0,
                                    style.dim.to_array(),
                                );
                                test_badges.push(crate::editor::test_runner::TestBadgeRegion {
                                    x1: badge_x,
                                    y1: row_y,
                                    x2: badge_x + badge_w,
                                    y2: row_y + line_h,
                                    test_index: i,
                                });
                            }
                        } else {
                            active_tests.clear();
                        }

                        pending_render_cache = Some((
                            active_tab,
                            buf_id,
                            render_lines,
                            current_change_id,
                            scroll_y_now,
                            hint_count_now,
                            dv.rect().w,
                            dv.rect().h,
                        ));

                        // Draw diagnostic underlines from LSP (only for LSP-handled files).
                        if subsystems.has_lsp()
                            && is_lsp_file
                            && let Some(diags) = lsp_state.diagnostics.get(&doc.path)
                        {
                            let line_h = style.line_height();
                            let gutter_w = dv.gutter_width;
                            let doc_x = dv.rect().x + gutter_w + style.padding_x;
                            let doc_y = dv.rect().y;
                            for diag in diags {
                                let color = match diag.severity {
                                    1 => style.error.to_array(),
                                    2 => style.warn.to_array(),
                                    _ => style.dim.to_array(),
                                };
                                let end_col = if diag.end_col == diag.start_col {
                                    diag.start_col + 1
                                } else {
                                    diag.end_col
                                };
                                // LSP lines are 0-based.
                                let y_pos = doc_y + (diag.start_line as f64) * line_h + line_h
                                    - 2.0
                                    - dv.scroll_y;
                                if y_pos < doc_y || y_pos > doc_y + dv.rect().h {
                                    continue;
                                }
                                use crate::editor::view::DrawContext as _;
                                let char_w = draw_ctx.font_width(style.code_font, "m");
                                let x1 = doc_x + diag.start_col as f64 * char_w - dv.scroll_x;
                                let x2 = doc_x + end_col as f64 * char_w - dv.scroll_x;
                                let w = (x2 - x1).max(char_w);
                                draw_ctx.draw_rect(x1, y_pos, w, 2.0, color);
                            }
                        }
                    }
                    // Git blame annotations (right-aligned, dimmed).
                    if subsystems.has_git() && git_blame_active && !git_blame_lines.is_empty() {
                        if let Some(doc) = docs.get(active_tab) {
                            let dv = &doc.view;
                            use crate::editor::view::DrawContext as _;
                            let line_h = style.line_height();
                            let first = ((dv.scroll_y / line_h).floor() as usize) + 1;
                            let vis = ((dv.rect().h / line_h).ceil() as usize) + 2;
                            let blame_color = style.dim.to_array();
                            let right_edge = dv.rect().x + dv.rect().w - style.padding_x;
                            for row in 0..vis {
                                let ln = first + row;
                                if ln > git_blame_lines.len() {
                                    break;
                                }
                                let annotation = &git_blame_lines[ln - 1];
                                let aw = draw_ctx.font_width(style.font, annotation);
                                let ax = (right_edge - aw).max(dv.rect().x + dv.gutter_width);
                                let ay = dv.rect().y + (ln as f64 - 1.0) * line_h - dv.scroll_y
                                    + (line_h - style.font_height) / 2.0;
                                if ay >= dv.rect().y
                                    && ay + style.font_height <= dv.rect().y + dv.rect().h
                                {
                                    draw_ctx.draw_text(style.font, annotation, ax, ay, blame_color);
                                }
                            }
                        }
                    }

                    // Inlay hints are injected into render_lines via build_render_lines.
                    // Reset clip before drawing minimap.
                    crate::editor::app_state::clip_init(width, height);
                    if minimap_visible {
                        use crate::editor::view::DrawContext as _;
                        let mm_x = width - minimap_w;
                        let mm_y = tab_h;
                        let mm_h = height - tab_h - terminal_h - status_h;
                        let mlh = 4.0_f64;
                        let text_padding = 4.0;
                        let usable_w = minimap_w - text_padding * 2.0;
                        let ref_cols = 80.0_f64;
                        let fixed_char_w = usable_w / ref_cols;
                        let block_height = (mlh * 0.6).max(1.0);
                        let block_y_pad = (mlh - block_height) / 2.0;

                        // Background.
                        let mut bg = style.background.to_array();
                        bg[3] = 230;
                        draw_ctx.draw_rect(mm_x, mm_y, minimap_w, mm_h, bg);
                        // Left border.
                        draw_ctx.draw_rect(mm_x, mm_y, 1.0, mm_h, [80, 80, 80, 60]);

                        let total_lines =
                            buffer::with_buffer(dv.buffer_id.unwrap_or(0), |b| Ok(b.lines.len()))
                                .unwrap_or(0);
                        if total_lines > 0 {
                            let doc_line_h = style.line_height();
                            let visible_lines = (dv.rect().h / doc_line_h).ceil() as usize;
                            let first_visible = (dv.scroll_y / doc_line_h).floor() as usize + 1;
                            let last_visible = first_visible + visible_lines;
                            let vis_center = (first_visible + last_visible) / 2;
                            let lines_that_fit = (mm_h / mlh).floor() as usize;

                            let minimap_start = if total_lines <= lines_that_fit {
                                1
                            } else {
                                let half = lines_that_fit / 2;
                                let start = vis_center.saturating_sub(half).max(1);
                                start.min(total_lines.saturating_sub(lines_that_fit) + 1)
                            };
                            let minimap_end = (minimap_start + lines_that_fit).min(total_lines + 1);

                            // Get compiled syntax for this file.
                            let ext = doc.path.rsplit('.').next().unwrap_or("");
                            let compiled = compiled_syntax_cache.get(ext).and_then(|o| o.as_ref());

                            // Draw colored blocks for each line.
                            let _ = buffer::with_buffer(dv.buffer_id.unwrap_or(0), |b| {
                                for line_idx in minimap_start..minimap_end {
                                    if line_idx > b.lines.len() {
                                        break;
                                    }
                                    let y_pos = mm_y
                                        + (line_idx - minimap_start) as f64 * mlh
                                        + block_y_pad;
                                    let raw = &b.lines[line_idx - 1];
                                    let text = raw.trim_end_matches('\n');
                                    if text.is_empty() {
                                        continue;
                                    }

                                    if let Some(syntax) = compiled {
                                        let toks = tokenizer::tokenize_line(syntax, raw);
                                        let mut x_off = 0.0;
                                        for t in &toks {
                                            let text_len = t.text.len();
                                            if text_len > 0 {
                                                let draw_len = if t.text.ends_with('\n') {
                                                    text_len - 1
                                                } else {
                                                    text_len
                                                };
                                                if draw_len > 0 {
                                                    let trimmed =
                                                        t.text.trim_start_matches([' ', '\t']);
                                                    let leading = text_len - trimmed.len();
                                                    let trimmed_draw =
                                                        draw_len.saturating_sub(leading);
                                                    if trimmed_draw > 0 {
                                                        let seg_x = (x_off
                                                            + leading as f64 * fixed_char_w)
                                                            .min(usable_w);
                                                        let seg_w = (trimmed_draw as f64
                                                            * fixed_char_w)
                                                            .min(usable_w - seg_x + text_padding);
                                                        if seg_w > 0.2 {
                                                            let mut color =
                                                                syntax_color(&t.token_type, &style);
                                                            color[3] = 130;
                                                            draw_ctx.draw_rect(
                                                                mm_x + text_padding + seg_x,
                                                                y_pos,
                                                                seg_w,
                                                                block_height,
                                                                color,
                                                            );
                                                        }
                                                    }
                                                }
                                                x_off += text_len as f64 * fixed_char_w;
                                            }
                                        }
                                    } else {
                                        let trimmed = text.trim_start();
                                        let leading = text.len() - trimmed.len();
                                        let draw_len =
                                            trimmed.len().min((usable_w / fixed_char_w) as usize);
                                        if draw_len > 0 {
                                            let seg_x = leading as f64 * fixed_char_w;
                                            let mut color = style.dim.to_array();
                                            color[3] = 130;
                                            draw_ctx.draw_rect(
                                                mm_x + text_padding + seg_x,
                                                y_pos,
                                                draw_len as f64 * fixed_char_w,
                                                block_height,
                                                color,
                                            );
                                        }
                                    }
                                }
                                Ok(())
                            });

                            // Viewport indicator.
                            if first_visible >= minimap_start && first_visible < minimap_end {
                                let ind_y = mm_y + (first_visible - minimap_start) as f64 * mlh;
                                let ind_h = (last_visible - first_visible) as f64 * mlh;
                                let clamped_h = ind_h.min(mm_h - (ind_y - mm_y));
                                let mut sel = style.selection.to_array();
                                sel[3] = 76;
                                draw_ctx.draw_rect(mm_x, ind_y, minimap_w, clamped_h, sel);
                            }
                        }
                    }
                } else {
                    empty_view.draw_native(&mut draw_ctx, &style);
                }
                crate::editor::app_state::clip_init(width, height);

                // Markdown preview pane (split, drawn to the right of the
                // editor view when enabled on the active doc). Runs after
                // the normal doc draw so it renders into its own rect.
                if let Some(doc) = docs.get_mut(active_tab) {
                    if doc.preview.enabled && doc.preview.rect.w > 0.0 {
                        if let Some(buf_id) = doc.view.buffer_id {
                            // Reparse the source when the buffer changes.
                            let cur_change_id =
                                buffer::with_buffer(buf_id, |b| Ok(b.change_id)).unwrap_or(0);
                            if cur_change_id != doc.preview.cached_change_id {
                                let text = buffer::with_buffer(buf_id, |b| Ok(b.lines.join("")))
                                    .unwrap_or_default();
                                doc.preview.blocks = crate::editor::markdown::parse(&text);
                                doc.preview.cached_change_id = cur_change_id;
                                doc.preview.layout.clear();
                                // Pre-tokenize every fenced code block with a
                                // resolvable `lang` so the preview can render
                                // it with syntax colours. Lookup reuses the
                                // editor's compiled-syntax cache keyed by file
                                // extension.
                                doc.preview.code_tokens = doc
                                    .preview
                                    .blocks
                                    .iter()
                                    .map(|blk| {
                                        let (lang, code_text) = match blk {
                                            crate::editor::markdown::Block::Code {
                                                lang: Some(l),
                                                text,
                                            } => (l.as_str(), text.as_str()),
                                            _ => return None,
                                        };
                                        let ext = markdown_lang_to_ext(lang);
                                        let ext_owned = ext.to_string();
                                        let pseudo = format!("f.{ext}");
                                        let compiled_opt = compiled_syntax_cache
                                            .entry(ext_owned.clone())
                                            .or_insert_with(|| {
                                                let entry =
                                                    crate::editor::syntax::match_syntax_entry(
                                                        &pseudo,
                                                        &syntax_index,
                                                    )?;
                                                let def = entry.load_full()?;
                                                tokenizer::compile_from_definition(&def).ok()
                                            })
                                            .as_ref()?;
                                        // Touch MRU so preview-only highlights
                                        // don't immediately get evicted.
                                        compiled_syntax_mru.retain(|e| e != &ext_owned);
                                        compiled_syntax_mru.insert(0, ext_owned);
                                        Some(
                                            code_text
                                                .split('\n')
                                                .map(|line| {
                                                    tokenizer::tokenize_line(compiled_opt, line)
                                                })
                                                .collect(),
                                        )
                                    })
                                    .collect();
                            }
                            let rect = doc.preview.rect;
                            // No smooth scroll: track the target directly so
                            // edits in the source that shrink `content_height`
                            // (and with it `max_scroll`) can't drive a multi-
                            // frame glide, which showed up as the preview
                            // auto-scrolling while the user typed.
                            let max_scroll = (doc.preview.content_height - rect.h).max(0.0);
                            doc.preview.target_scroll_y =
                                doc.preview.target_scroll_y.clamp(0.0, max_scroll);
                            doc.preview.scroll_y = doc.preview.target_scroll_y;
                            // Divider between editor and preview.
                            use crate::editor::view::DrawContext as _;
                            draw_ctx.draw_rect(
                                rect.x,
                                rect.y,
                                style.divider_size.max(1.0),
                                rect.h,
                                style.divider.to_array(),
                            );
                            let pane_x = rect.x + style.divider_size.max(1.0);
                            let pane_w = rect.w - style.divider_size.max(1.0);
                            crate::editor::markdown_preview::draw(
                                &mut draw_ctx,
                                &mut doc.preview,
                                &style,
                                pane_x,
                                rect.y,
                                pane_w,
                                rect.h,
                            );
                        }
                    }
                }
                crate::editor::app_state::clip_init(width, height);

                // Draw terminal panel.
                if subsystems.has_terminal() && terminal.visible {
                    use crate::editor::view::DrawContext as _;
                    // Keep the terminal palette in sync with the live theme.
                    let (term_palette, term_default_fg) =
                        crate::editor::terminal_panel::theme_terminal_palette(&style);
                    terminal.set_palette(term_palette, term_default_fg);
                    let term_y = height - terminal_h - status_h;
                    let term_x = sidebar_w;
                    let term_w = width - sidebar_w;
                    // Divider at top of terminal.
                    draw_ctx.draw_rect(
                        term_x,
                        term_y,
                        term_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(
                        term_x,
                        term_y + style.divider_size,
                        term_w,
                        terminal_h - style.divider_size,
                        style.background.to_array(),
                    );
                    // Resize terminal buffer to match panel dimensions.
                    let tab_bar_h_for_resize = if !terminal.terminals.is_empty() {
                        style.font_height + style.padding_y * 3.0
                    } else {
                        0.0
                    };
                    let char_h_resize = style.line_height();
                    let char_w_resize = draw_ctx.font_width(style.code_font, "m");
                    if char_w_resize > 0.0 && char_h_resize > 0.0 {
                        let avail_h = terminal_h
                            - style.divider_size
                            - tab_bar_h_for_resize
                            - style.padding_y * 2.0;
                        let new_cols =
                            ((term_w - style.padding_x * 2.0) / char_w_resize).max(1.0) as usize;
                        let new_rows = (avail_h / char_h_resize).max(1.0) as usize;
                        if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                            if inst.last_pty_size != (new_cols, new_rows) {
                                inst.tbuf.resize(new_cols, new_rows);
                                inst.inner.resize(new_cols as u16, new_rows as u16);
                                inst.last_pty_size = (new_cols, new_rows);
                            }
                        }
                    }
                    // Draw terminal title/tab bar using the same layout as the doc tab bar.
                    let tab_bar_h = if !terminal.terminals.is_empty() {
                        let tbh = style.font_height + style.padding_y * 3.0;
                        let accent_h = 3.0;
                        let tby = term_y + style.divider_size;
                        draw_ctx.draw_rect(term_x, tby, term_w, tbh, style.background2.to_array());
                        let close_w = draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                        let mut tx = term_x;
                        for (i, inst) in terminal.terminals.iter().enumerate() {
                            let label = &inst.title;
                            let label_w = draw_ctx.font_width(style.font, label);
                            let tw = label_w + style.padding_x * 2.0 + close_w;
                            let bg = if i == terminal.active {
                                style.background.to_array()
                            } else {
                                style.background2.to_array()
                            };
                            let fg = if i == terminal.active {
                                style.text.to_array()
                            } else {
                                style.dim.to_array()
                            };
                            draw_ctx.draw_rect(tx, tby + accent_h, tw, tbh - accent_h, bg);
                            if i == terminal.active {
                                draw_ctx.draw_rect(tx, tby, tw, accent_h, style.accent.to_array());
                            }
                            let text_y =
                                tby + accent_h + (tbh - accent_h - style.font_height) / 2.0;
                            draw_ctx.draw_text(style.font, label, tx + style.padding_x, text_y, fg);
                            let close_x = tx + tw - close_w;
                            let close_hovered = mouse_y >= tby
                                && mouse_y < tby + tbh
                                && mouse_x >= close_x
                                && mouse_x < close_x + close_w;
                            if close_hovered {
                                draw_ctx.draw_rect(
                                    close_x,
                                    tby + accent_h,
                                    close_w,
                                    tbh - accent_h,
                                    style.line_highlight.to_array(),
                                );
                            }
                            let close_color = if close_hovered {
                                style.text.to_array()
                            } else {
                                style.dim.to_array()
                            };
                            draw_ctx.draw_text(
                                style.icon_font,
                                "C",
                                close_x + style.padding_x * 0.5,
                                tby + accent_h
                                    + (tbh - accent_h - draw_ctx.font_height(style.icon_font))
                                        / 2.0,
                                close_color,
                            );
                            draw_ctx.draw_rect(
                                tx + tw,
                                tby + style.padding_y * 0.5,
                                style.divider_size,
                                tbh - style.padding_y,
                                style.dim.to_array(),
                            );
                            tx += tw + style.divider_size;
                        }
                        draw_ctx.draw_rect(
                            term_x,
                            tby + tbh - style.divider_size,
                            term_w,
                            style.divider_size,
                            style.divider.to_array(),
                        );
                        tbh
                    } else {
                        0.0
                    };
                    // Draw active terminal buffer text using TerminalBufferInner cell grid.
                    if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                        let char_h = style.line_height();
                        let char_w = draw_ctx.font_width(style.code_font, "m");
                        let ty_start = term_y + style.divider_size + tab_bar_h + 2.0;
                        let visible_h = (term_y + terminal_h - ty_start - style.padding_y).max(0.0);
                        let rows_visible = (visible_h / char_h).floor().max(1.0) as usize;

                        let cap = inst.tbuf.history_len() as f64;
                        inst.scrollback_target = inst.scrollback_target.clamp(0.0, cap);
                        let diff = inst.scrollback_target - inst.scrollback;
                        if diff.abs() >= 0.5 {
                            inst.scrollback += diff * 0.35;
                            crate::window::force_invalidate();
                        } else if inst.scrollback != inst.scrollback_target {
                            inst.scrollback = inst.scrollback_target;
                        }
                        let scrollback_rows = inst.scrollback.round().max(0.0).min(cap) as usize;
                        let rows_data = inst.tbuf.visible_rows(rows_visible, scrollback_rows);

                        // Normalized selection range for this frame.
                        let sel_range = match (inst.sel_start, inst.sel_end) {
                            (Some(s), Some(e)) => {
                                crate::editor::terminal_panel::normalized_selection(s, e)
                            }
                            _ => None,
                        };

                        let cur_row_1 = inst.tbuf.cursor_row();
                        let cur_col_1 = inst.tbuf.cursor_col();
                        let cur_visible_row = if scrollback_rows == 0 {
                            Some(cur_row_1.saturating_sub(1))
                        } else if scrollback_rows < rows_visible {
                            Some(rows_visible - scrollback_rows + cur_row_1.saturating_sub(1))
                                .filter(|r| *r < rows_visible)
                        } else {
                            None
                        };

                        for (row_idx, row) in rows_data.iter().enumerate() {
                            let ry = ty_start + row_idx as f64 * char_h;
                            if ry + char_h < term_y || ry > term_y + terminal_h {
                                continue;
                            }
                            // Batch adjacent chars with same fg for efficient rendering.
                            let mut run_text = String::new();
                            let mut run_x = term_x + style.padding_x;
                            let mut run_fg: [u8; 4] = style.text.to_array();
                            let mut cx = term_x + style.padding_x;

                            for (col_idx, cell) in row.iter().enumerate() {
                                let ch = char::from_u32(cell.ch).unwrap_or(' ');
                                let fg = crate::editor::terminal::unpack_color(cell.fg)
                                    .unwrap_or(style.text.to_array());
                                let bg = crate::editor::terminal::unpack_color(cell.bg);

                                // Selection highlight for this cell.
                                let in_sel = match sel_range {
                                    Some((a, b)) => {
                                        (row_idx > a.0 && row_idx < b.0)
                                            || (row_idx == a.0
                                                && row_idx == b.0
                                                && col_idx >= a.1
                                                && col_idx < b.1)
                                            || (row_idx == a.0 && row_idx != b.0 && col_idx >= a.1)
                                            || (row_idx == b.0 && row_idx != a.0 && col_idx < b.1)
                                    }
                                    None => false,
                                };
                                if in_sel {
                                    draw_ctx.draw_rect(
                                        cx,
                                        ry,
                                        char_w,
                                        char_h,
                                        style.selection.to_array(),
                                    );
                                }

                                // Draw bg if non-zero (and not already selected).
                                if !in_sel {
                                    if let Some(bg_color) = bg {
                                        if bg_color[3] > 0 && bg_color != [0, 0, 0, 255] {
                                            draw_ctx.draw_rect(cx, ry, char_w, char_h, bg_color);
                                        }
                                    }
                                }

                                // Batch text runs with same fg color.
                                if fg != run_fg && !run_text.is_empty() {
                                    draw_ctx.draw_text(
                                        style.code_font,
                                        &run_text,
                                        run_x,
                                        ry,
                                        run_fg,
                                    );
                                    run_text.clear();
                                    run_x = cx;
                                    run_fg = fg;
                                }
                                if run_text.is_empty() {
                                    run_x = cx;
                                    run_fg = fg;
                                }
                                run_text.push(ch);

                                if terminal.focused
                                    && Some(row_idx) == cur_visible_row
                                    && col_idx == cur_col_1.saturating_sub(1)
                                {
                                    draw_ctx.draw_rect(cx, ry, char_w, char_h, [200, 200, 200, 80]);
                                }
                                cx += char_w;
                            }
                            // Flush remaining text run.
                            if !run_text.is_empty() {
                                draw_ctx.draw_text(style.code_font, &run_text, run_x, ry, run_fg);
                            }
                        }

                        // Scrollbar (shown only when there is history).
                        if cap > 0.0 {
                            let sb_w = style.scrollbar_size.max(6.0);
                            let sb_x = term_x + term_w - sb_w;
                            let sb_y = ty_start;
                            let sb_h = char_h * rows_visible as f64;
                            draw_ctx.draw_rect(
                                sb_x,
                                sb_y,
                                sb_w,
                                sb_h,
                                style.scrollbar_track.to_array(),
                            );
                            let total = cap + rows_visible as f64;
                            let ratio = (rows_visible as f64 / total).clamp(0.0, 1.0);
                            let min_thumb = sb_w * 2.0;
                            let thumb_h = (sb_h * ratio).max(min_thumb).min(sb_h);
                            // scrollback = 0 -> thumb at bottom of track
                            // scrollback = cap -> thumb at top.
                            let pos_from_top = (cap - inst.scrollback) / cap;
                            let thumb_y = sb_y + pos_from_top * (sb_h - thumb_h);
                            draw_ctx.draw_rect(
                                sb_x,
                                thumb_y,
                                sb_w,
                                thumb_h,
                                style.scrollbar.to_array(),
                            );
                        }
                    }
                }

                status_view.draw_native(&mut draw_ctx, &style);

                // Draw find bar (and optionally replace bar) at the top of the editor,
                // just below the tab and breadcrumb bars, so transient UX is consistent.
                // The bar spans only the active editor's column (not the sidebar/minimap)
                // so the user's eye stays anchored to the document being searched.
                if find_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let row_h = style.font_height + style.padding_y * 2.0;
                    let total_rows = if replace_active { 3.0 } else { 2.0 };
                    let bar_x = sidebar_w;
                    let bar_w = (width - sidebar_w - minimap_w).max(0.0);
                    let bar_y = tab_h + breadcrumb_h;
                    let bar_total_h = row_h * total_rows;

                    draw_ctx.draw_rect(
                        bar_x,
                        bar_y,
                        bar_w,
                        bar_total_h,
                        style.background3.to_array(),
                    );
                    draw_ctx.draw_rect(
                        bar_x,
                        bar_y,
                        bar_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(
                        bar_x,
                        bar_y + bar_total_h - style.divider_size,
                        bar_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Row 1: Find input + count indicator on the right.
                    let find_cursor = if !find_focus_on_replace { "_" } else { "" };
                    let find_label = format!("Find: {find_query}{find_cursor}");
                    draw_ctx.draw_text(
                        style.font,
                        &find_label,
                        bar_x + style.padding_x,
                        bar_y + style.padding_y,
                        style.text.to_array(),
                    );
                    let count_label = if find_query.is_empty() {
                        String::new()
                    } else if find_matches.is_empty() {
                        "0/0".to_string()
                    } else {
                        let cur = find_current.map(|i| i + 1).unwrap_or(0);
                        format!("{cur}/{}", find_matches.len())
                    };
                    if !count_label.is_empty() {
                        let cw = draw_ctx.font_width(style.font, &count_label);
                        draw_ctx.draw_text(
                            style.font,
                            &count_label,
                            bar_x + bar_w - cw - style.padding_x,
                            bar_y + style.padding_y,
                            if find_matches.is_empty() {
                                style.error.to_array()
                            } else {
                                style.dim.to_array()
                            },
                        );
                    }

                    // Optional Row 2: Replace input.
                    let mut next_row_y = bar_y + row_h;
                    if replace_active {
                        let replace_y = next_row_y;
                        draw_ctx.draw_rect(
                            bar_x,
                            replace_y,
                            bar_w,
                            style.divider_size,
                            style.divider.to_array(),
                        );
                        let repl_cursor = if find_focus_on_replace { "_" } else { "" };
                        let repl_label = format!(
                            "Replace: {replace_query}{repl_cursor}  (Ctrl+Enter replace  Ctrl+Shift+Enter all)"
                        );
                        draw_ctx.draw_text(
                            style.font,
                            &repl_label,
                            bar_x + style.padding_x,
                            replace_y + style.padding_y,
                            style.text.to_array(),
                        );
                        next_row_y += row_h;
                    }

                    // Final row: keybinding hints with on/off indicators for the toggles.
                    let hint_y = next_row_y;
                    draw_ctx.draw_rect(
                        bar_x,
                        hint_y,
                        bar_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let mark = |on: bool| if on { "[x]" } else { "[ ]" };
                    let hint = format!(
                        "Alt+R Regex {}  Alt+W Word {}  Alt+I Case {}  Alt+S Sel {}   F3 Next  Shift+F3 Prev  Esc Close",
                        mark(find_use_regex),
                        mark(find_whole_word),
                        mark(find_case_insensitive),
                        mark(find_in_selection),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &hint,
                        bar_x + style.padding_x,
                        hint_y + style.padding_y,
                        style.dim.to_array(),
                    );
                }

                // Loading overlay for large file background loads.
                if let Some(job) = load_job.as_ref() {
                    use crate::editor::view::DrawContext as _;
                    crate::editor::app_state::clip_init(width, height);
                    // Dim background.
                    draw_ctx.draw_rect(0.0, 0.0, width, height, [0, 0, 0, 160]);
                    // Centered dialog.
                    let dlg_w = 520.0_f64.min(width - 40.0);
                    let dlg_h = style.font_height * 3.5 + style.padding_y * 4.0;
                    let dlg_x = (width - dlg_w) / 2.0;
                    let dlg_y = (height - dlg_h) / 2.0;
                    draw_ctx.draw_rect(
                        dlg_x - 1.0,
                        dlg_y - 1.0,
                        dlg_w + 2.0,
                        dlg_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(dlg_x, dlg_y, dlg_w, dlg_h, style.background3.to_array());
                    // Title.
                    let title = format!("Loading {}", job.name);
                    draw_ctx.draw_text(
                        style.font,
                        &title,
                        dlg_x + style.padding_x,
                        dlg_y + style.padding_y,
                        style.text.to_array(),
                    );
                    // Progress numbers.
                    let bytes = job.bytes_read.load(std::sync::atomic::Ordering::Relaxed);
                    let pct = if job.total_bytes > 0 {
                        (bytes as f64 / job.total_bytes as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let fmt_mb = |b: u64| format!("{:.1} MB", b as f64 / (1024.0 * 1024.0));
                    let status = format!(
                        "{} / {}  ({:.0}%)",
                        fmt_mb(bytes),
                        fmt_mb(job.total_bytes),
                        pct * 100.0,
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &status,
                        dlg_x + style.padding_x,
                        dlg_y + style.padding_y * 2.0 + style.font_height,
                        style.dim.to_array(),
                    );
                    // Progress bar.
                    let bar_x = dlg_x + style.padding_x;
                    let bar_y = dlg_y + dlg_h - style.padding_y - style.font_height / 2.0;
                    let bar_w = dlg_w - style.padding_x * 2.0;
                    let bar_h = style.font_height / 2.0;
                    draw_ctx.draw_rect(bar_x, bar_y, bar_w, bar_h, style.divider.to_array());
                    draw_ctx.draw_rect(bar_x, bar_y, bar_w * pct, bar_h, style.accent.to_array());
                }

                // Nag bar takes priority over all overlays.
                if let Nag::UnsavedChanges { message, .. } = &nag {
                    cmdview_active = false;
                    palette_active = false;
                    completion.hide();
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    // Semi-transparent overlay dims the entire editor.
                    draw_ctx.draw_rect(0.0, 0.0, width, height, [0, 0, 0, 120]);
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    draw_ctx.draw_text(
                        style.font,
                        message,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                    // Draw option buttons.
                    let msg_w = draw_ctx.font_width(style.font, message);
                    let btn_y = style.padding_y * 0.5;
                    let btn_h = style.font_height + style.padding_y;
                    let btn_pad = style.padding_x;
                    let mut bx = style.padding_x + msg_w + btn_pad * 2.0;
                    for label in &["Yes", "No"] {
                        let lw = draw_ctx.font_width(style.font, label) + btn_pad * 2.0;
                        draw_ctx.draw_rect(bx, btn_y, lw, btn_h, style.nagbar_text.to_array());
                        draw_ctx.draw_text(
                            style.font,
                            label,
                            bx + btn_pad,
                            btn_y + style.padding_y * 0.5,
                            style.nagbar.to_array(),
                        );
                        bx += lw + btn_pad;
                    }
                }

                // Warn once per session per codepoint when a drawn character
                // is covered by no configured or installed system font.
                let uncovered = crate::renderer::take_uncovered();
                if let Some(&cp) = uncovered.first() {
                    let more = match uncovered.len() {
                        1 => String::new(),
                        n => format!(" and {} more", n - 1),
                    };
                    let msg = format!(
                        "No installed font covers U+{cp:04X}{more} -- install a font for this script or set fonts.code.paths in config"
                    );
                    log::warn!("{msg}");
                    info_message = Some((msg, Instant::now()));
                }

                // Draw info message (auto-dismiss after 3s, or on any key).
                if let Some((ref msg, at)) = info_message {
                    if at.elapsed().as_secs() >= 3 {
                        info_message = None;
                    } else {
                        crate::editor::app_state::clip_init(width, height);
                        use crate::editor::view::DrawContext as _;
                        let bar_h = style.font_height + style.padding_y * 2.0;
                        draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.accent.to_array());
                        let ty = (bar_h - style.font_height) / 2.0;
                        draw_ctx.draw_text(
                            style.font,
                            msg,
                            style.padding_x,
                            ty,
                            [255, 255, 255, 255],
                        );
                    }
                }

                // Draw "create missing directory?" confirmation bar.
                if let Nag::CreateDir { parent, .. } = &nag {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    let msg = format!(
                        "Directory does not exist: {parent}. Create it and save?  [Y]es  [N]o"
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &msg,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                }

                // Draw "overwrite existing file?" confirmation bar.
                if let Nag::OverwriteFile { save_path, .. } = &nag {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    let msg = format!("{save_path} already exists. Overwrite?  [Y]es  [N]o");
                    draw_ctx.draw_text(
                        style.font,
                        &msg,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                }

                // Draw "no extension detected?" confirmation bar.
                if let Nag::NoExtension { save_path, .. } = &nag {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    let msg =
                        format!("No extension detected ({save_path}). Save anyway?  [Y]es  [N]o");
                    draw_ctx.draw_text(
                        style.font,
                        &msg,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                }

                // Draw "delete file?" confirmation bar.
                if let Nag::DeleteFile { path } = &nag {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    let msg = format!("Delete {path}?  [Y]es  [N]o");
                    draw_ctx.draw_text(
                        style.font,
                        &msg,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                }

                // Draw reload nag bar if active.
                if let Nag::ReloadFromDisk { path } = &nag {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let bar_h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rect(0.0, 0.0, width, bar_h, style.nagbar.to_array());
                    let msg = format!("File changed on disk: {path}. Reload?  [Y]es  [N]o");
                    draw_ctx.draw_text(
                        style.font,
                        &msg,
                        style.padding_x,
                        style.padding_y,
                        style.nagbar_text.to_array(),
                    );
                }

                // Draw command palette if active.
                if palette_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let pal_w = (width * 0.5).max(400.0).min(width - 20.0);
                    let pal_x = (width - pal_w) / 2.0;
                    let pal_y = style.padding_y * 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_visible = 12usize;
                    let visible = palette_results.len().min(max_visible);
                    let pal_h = line_h * (visible as f64 + 1.0) + style.padding_y * 2.0;

                    let pal_r = 10.0;
                    draw_ctx.draw_rounded_rect(
                        pal_x - 1.0,
                        pal_y - 1.0,
                        pal_w + 2.0,
                        pal_h + 2.0,
                        pal_r,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rounded_rect(pal_x, pal_y, pal_w, pal_h, pal_r, style.background3.to_array());

                    let input_y = pal_y + style.padding_y;
                    draw_ctx.draw_text(
                        style.font,
                        &format!("> {palette_query}_"),
                        pal_x + style.padding_x,
                        input_y,
                        style.text.to_array(),
                    );
                    draw_ctx.draw_rect(
                        pal_x,
                        input_y + line_h,
                        pal_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Scroll the visible window so palette_selected stays in view.
                    let scroll_off = if palette_selected >= max_visible {
                        palette_selected - max_visible + 1
                    } else {
                        0
                    };
                    for (i, (_, display)) in palette_results
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_visible)
                    {
                        let display_idx = i - scroll_off;
                        let ry =
                            input_y + line_h + style.divider_size + display_idx as f64 * line_h;
                        if i == palette_selected {
                            let inner_pad = style.padding_x * 0.5;
                            draw_ctx.draw_rounded_rect(
                                pal_x + inner_pad,
                                ry + 1.0,
                                pal_w - inner_pad * 2.0,
                                line_h - 2.0,
                                6.0,
                                style.selection.to_array(),
                            );
                        }
                        let color = if i == palette_selected {
                            style.accent.to_array()
                        } else {
                            style.text.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            display,
                            pal_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            color,
                        );
                    }
                }

                // Draw theme picker if active.
                if theme_picker_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let pal_w = (width * 0.4).max(300.0).min(width - 20.0);
                    let pal_x = (width - pal_w) / 2.0;
                    let pal_y = style.padding_y * 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_visible = 12usize;
                    let visible = theme_picker_results.len().min(max_visible);
                    let pal_h = line_h * (visible as f64 + 1.0) + style.padding_y * 2.0;

                    let pal_r = 10.0;
                    draw_ctx.draw_rounded_rect(
                        pal_x - 1.0,
                        pal_y - 1.0,
                        pal_w + 2.0,
                        pal_h + 2.0,
                        pal_r,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rounded_rect(pal_x, pal_y, pal_w, pal_h, pal_r, style.background3.to_array());

                    let input_y = pal_y + style.padding_y;
                    draw_ctx.draw_text(
                        style.font,
                        &format!("> {theme_picker_query}_"),
                        pal_x + style.padding_x,
                        input_y,
                        style.text.to_array(),
                    );
                    draw_ctx.draw_rect(
                        pal_x,
                        input_y + line_h,
                        pal_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Scroll the visible window so theme_picker_selected stays in view.
                    let scroll_off = if theme_picker_selected >= max_visible {
                        theme_picker_selected - max_visible + 1
                    } else {
                        0
                    };
                    for (i, (_, display)) in theme_picker_results
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_visible)
                    {
                        let display_idx = i - scroll_off;
                        let ry =
                            input_y + line_h + style.divider_size + display_idx as f64 * line_h;
                        if i == theme_picker_selected {
                            let inner_pad = style.padding_x * 0.5;
                            draw_ctx.draw_rounded_rect(
                                pal_x + inner_pad,
                                ry + 1.0,
                                pal_w - inner_pad * 2.0,
                                line_h - 2.0,
                                6.0,
                                style.selection.to_array(),
                            );
                        }
                        let color = if i == theme_picker_selected {
                            style.accent.to_array()
                        } else {
                            style.text.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            display,
                            pal_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            color,
                        );
                    }
                }

                // Draw project search overlay.
                if subsystems.has_find_in_files() && project_search_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let ps_w = (width * 0.6).max(500.0).min(width - 20.0);
                    let ps_x = (width - ps_w) / 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_visible = 15usize;
                    let visible_count = project_search_results.len().min(max_visible);
                    // Title + input + hint + results.
                    let ps_h = line_h * (visible_count as f64 + 3.0) + style.padding_y * 2.0;
                    let ps_y = style.padding_y * 2.0;

                    draw_ctx.draw_rect(
                        ps_x - 1.0,
                        ps_y - 1.0,
                        ps_w + 2.0,
                        ps_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(ps_x, ps_y, ps_w, ps_h, style.background3.to_array());

                    // Title bar.
                    let title_y = ps_y + style.padding_y;
                    draw_ctx.draw_text(
                        style.font,
                        "Find in Files",
                        ps_x + style.padding_x,
                        title_y,
                        style.accent.to_array(),
                    );
                    let match_count = format!("  ({} matches)", project_search_results.len());
                    let title_w = draw_ctx.font_width(style.font, "Find in Files");
                    draw_ctx.draw_text(
                        style.font,
                        &match_count,
                        ps_x + style.padding_x + title_w,
                        title_y,
                        style.dim.to_array(),
                    );
                    draw_ctx.draw_rect(
                        ps_x,
                        title_y + line_h,
                        ps_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Input line.
                    let input_y = title_y + line_h;
                    let label = "Search: ";
                    let label_w = draw_ctx.font_width(style.font, label);
                    draw_ctx.draw_text(
                        style.font,
                        label,
                        ps_x + style.padding_x,
                        input_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &format!("{}_", &project_search_query),
                        ps_x + style.padding_x + label_w + style.padding_x,
                        input_y,
                        style.text.to_array(),
                    );

                    // Toggle hints.
                    let hint_y = input_y + line_h;
                    draw_ctx.draw_rect(
                        ps_x,
                        hint_y,
                        ps_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let mark = |on: bool| if on { "[x]" } else { "[ ]" };
                    let hint = format!(
                        "Alt+R Regex {}  Alt+W Word {}  Alt+I Case {}   Enter open  Esc close",
                        mark(project_use_regex),
                        mark(project_whole_word),
                        mark(project_case_insensitive),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &hint,
                        ps_x + style.padding_x,
                        hint_y + style.padding_y * 0.5,
                        style.dim.to_array(),
                    );

                    // Divider below hints.
                    let results_start_y = hint_y + line_h;
                    draw_ctx.draw_rect(
                        ps_x,
                        results_start_y,
                        ps_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Scroll offset so selected item is visible.
                    let scroll_off = if project_search_selected >= max_visible {
                        project_search_selected - max_visible + 1
                    } else {
                        0
                    };

                    // Results list.
                    for (i, (path, line_num, text)) in project_search_results
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_visible)
                    {
                        let display_idx = i - scroll_off;
                        let ry = results_start_y + style.divider_size + display_idx as f64 * line_h;
                        if i == project_search_selected {
                            draw_ctx.draw_rect(ps_x, ry, ps_w, line_h, style.selection.to_array());
                        }
                        // Show path:line then the matched text.
                        let location = format!("{path}:{line_num}");
                        let loc_color = if i == project_search_selected {
                            style.accent.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            &location,
                            ps_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            loc_color,
                        );
                        let loc_w = draw_ctx.font_width(style.font, &location);
                        let text_color = style.text.to_array();
                        let max_text_w = ps_w - style.padding_x * 3.0 - loc_w;
                        let truncated: String = if max_text_w > 0.0 {
                            let char_w = draw_ctx.font_width(style.font, "m");
                            let max_chars = (max_text_w / char_w).floor() as usize;
                            text.chars().take(max_chars).collect()
                        } else {
                            String::new()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            &format!("  {truncated}"),
                            ps_x + style.padding_x + loc_w,
                            ry + style.padding_y / 2.0,
                            text_color,
                        );
                    }
                }

                // Draw project replace overlay.
                if subsystems.has_find_in_files() && project_replace_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let pr_w = (width * 0.6).max(500.0).min(width - 20.0);
                    let pr_x = (width - pr_w) / 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_visible = 12usize;
                    let visible_count = project_replace_results.len().min(max_visible);
                    // Title + search + replace + toggles + hint + results.
                    let pr_h = line_h * (visible_count as f64 + 5.0) + style.padding_y * 2.0;
                    let pr_y = style.padding_y * 2.0;

                    draw_ctx.draw_rect(
                        pr_x - 1.0,
                        pr_y - 1.0,
                        pr_w + 2.0,
                        pr_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(pr_x, pr_y, pr_w, pr_h, style.background3.to_array());

                    // Title bar.
                    let title_y = pr_y + style.padding_y;
                    draw_ctx.draw_text(
                        style.font,
                        "Replace in Files",
                        pr_x + style.padding_x,
                        title_y,
                        style.accent.to_array(),
                    );
                    let match_label = format!("  ({} matches)", project_replace_results.len());
                    let tw = draw_ctx.font_width(style.font, "Replace in Files");
                    draw_ctx.draw_text(
                        style.font,
                        &match_label,
                        pr_x + style.padding_x + tw,
                        title_y,
                        style.dim.to_array(),
                    );
                    draw_ctx.draw_rect(
                        pr_x,
                        title_y + line_h,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Search input.
                    let row1_y = title_y + line_h;
                    let search_cursor = if !project_replace_focus_on_replace {
                        "_"
                    } else {
                        ""
                    };
                    let search_label = "Search: ";
                    let sl_w = draw_ctx.font_width(style.font, search_label);
                    draw_ctx.draw_text(
                        style.font,
                        search_label,
                        pr_x + style.padding_x,
                        row1_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &format!("{project_replace_search}{search_cursor}"),
                        pr_x + style.padding_x + sl_w + style.padding_x,
                        row1_y,
                        style.text.to_array(),
                    );

                    // Replace input.
                    let row2_y = row1_y + line_h;
                    draw_ctx.draw_rect(
                        pr_x,
                        row2_y,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let replace_cursor = if project_replace_focus_on_replace {
                        "_"
                    } else {
                        ""
                    };
                    let rl = "Replace: ";
                    let rl_w = draw_ctx.font_width(style.font, rl);
                    draw_ctx.draw_text(
                        style.font,
                        rl,
                        pr_x + style.padding_x,
                        row2_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &format!("{project_replace_with}{replace_cursor}"),
                        pr_x + style.padding_x + rl_w + style.padding_x,
                        row2_y,
                        style.text.to_array(),
                    );

                    // Toggle hints.
                    let toggles_y = row2_y + line_h;
                    draw_ctx.draw_rect(
                        pr_x,
                        toggles_y,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let mark = |on: bool| if on { "[x]" } else { "[ ]" };
                    let toggle_hint = format!(
                        "Alt+R Regex {}  Alt+W Word {}  Alt+I Case {}",
                        mark(project_use_regex),
                        mark(project_whole_word),
                        mark(project_case_insensitive),
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &toggle_hint,
                        pr_x + style.padding_x,
                        toggles_y + style.padding_y * 0.5,
                        style.dim.to_array(),
                    );

                    // Action hint row.
                    let hint_y = toggles_y + line_h;
                    draw_ctx.draw_rect(
                        pr_x,
                        hint_y,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let hint =
                        "Tab switch fields  Enter preview  Ctrl+Enter replace all  Esc close";
                    draw_ctx.draw_text(
                        style.font,
                        hint,
                        pr_x + style.padding_x,
                        hint_y + style.padding_y * 0.5,
                        style.dim.to_array(),
                    );

                    // Results preview.
                    let results_y = hint_y + line_h;
                    draw_ctx.draw_rect(
                        pr_x,
                        results_y,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(
                        pr_x,
                        results_y,
                        pr_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let scroll_off = if project_replace_selected >= max_visible {
                        project_replace_selected - max_visible + 1
                    } else {
                        0
                    };
                    for (i, (path, line_num, text)) in project_replace_results
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_visible)
                    {
                        let di = i - scroll_off;
                        let ry = results_y + style.divider_size + di as f64 * line_h;
                        if i == project_replace_selected {
                            draw_ctx.draw_rect(pr_x, ry, pr_w, line_h, style.selection.to_array());
                        }
                        let location = format!("{path}:{line_num}");
                        let loc_color = if i == project_replace_selected {
                            style.accent.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            &location,
                            pr_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            loc_color,
                        );
                        let loc_w = draw_ctx.font_width(style.font, &location);
                        let max_text_w = pr_w - style.padding_x * 3.0 - loc_w;
                        if max_text_w > 0.0 {
                            let char_w = draw_ctx.font_width(style.font, "m");
                            let max_chars = (max_text_w / char_w).floor() as usize;
                            let truncated: String = text.chars().take(max_chars).collect();
                            draw_ctx.draw_text(
                                style.font,
                                &format!("  {truncated}"),
                                pr_x + style.padding_x + loc_w,
                                ry + style.padding_y / 2.0,
                                style.text.to_array(),
                            );
                        }
                    }
                }

                // Draw git status overlay.
                if subsystems.has_git() && git_status_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let gs_w = (width * 0.5).max(400.0).min(width - 20.0);
                    let gs_x = (width - gs_w) / 2.0;
                    let gs_y = style.padding_y * 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_vis = 20usize;
                    let vis = git_status_entries.len().min(max_vis);
                    let gs_h = line_h * (vis as f64 + 1.0) + style.padding_y * 2.0;
                    draw_ctx.draw_rect(
                        gs_x - 1.0,
                        gs_y - 1.0,
                        gs_w + 2.0,
                        gs_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(gs_x, gs_y, gs_w, gs_h, style.background3.to_array());
                    let input_y = gs_y + style.padding_y;
                    let title = format!(
                        "Git Status  ({} changed)  [R] refresh  [Enter] open  [Esc] close",
                        git_status_entries.len()
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &title,
                        gs_x + style.padding_x,
                        input_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_rect(
                        gs_x,
                        input_y + line_h,
                        gs_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let scroll_off = if git_status_selected >= max_vis {
                        git_status_selected - max_vis + 1
                    } else {
                        0
                    };
                    for (i, (code, _path, display)) in git_status_entries
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_vis)
                    {
                        let di = i - scroll_off;
                        let ry = input_y + line_h + style.divider_size + di as f64 * line_h;
                        if i == git_status_selected {
                            draw_ctx.draw_rect(gs_x, ry, gs_w, line_h, style.selection.to_array());
                        }
                        let color = match code.as_str() {
                            "M" | "MM" => style.warn.to_array(),
                            "A" | "AM" => style.good.to_array(),
                            "D" => style.error.to_array(),
                            "?" | "??" => style.dim.to_array(),
                            _ => style.text.to_array(),
                        };
                        draw_ctx.draw_text(
                            style.font,
                            display,
                            gs_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            color,
                        );
                    }
                }

                // Draw git log overlay.
                if code_action_active && !code_actions.is_empty() {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let ca_w = (width * 0.5).max(400.0).min(width - 20.0);
                    let ca_x = (width - ca_w) / 2.0;
                    let ca_y = style.padding_y * 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_vis = 15usize;
                    let vis = code_actions.len().min(max_vis);
                    let ca_h = line_h * (vis as f64 + 1.0) + style.padding_y * 2.0;
                    draw_ctx.draw_rect(
                        ca_x - 1.0,
                        ca_y - 1.0,
                        ca_w + 2.0,
                        ca_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(ca_x, ca_y, ca_w, ca_h, style.background3.to_array());
                    let input_y = ca_y + style.padding_y;
                    let title = format!(
                        "Code Actions  ({})  [Enter] apply  [Esc] close",
                        code_actions.len()
                    );
                    draw_ctx.draw_text(
                        style.font,
                        &title,
                        ca_x + style.padding_x,
                        input_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_rect(
                        ca_x,
                        input_y + line_h,
                        ca_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let scroll_off = if code_action_selected >= max_vis {
                        code_action_selected - max_vis + 1
                    } else {
                        0
                    };
                    for (i, (action_title, _)) in code_actions
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_vis)
                    {
                        let di = i - scroll_off;
                        let ry = input_y + line_h + style.divider_size + di as f64 * line_h;
                        if i == code_action_selected {
                            draw_ctx.draw_rect(ca_x, ry, ca_w, line_h, style.selection.to_array());
                        }
                        let color = if i == code_action_selected {
                            style.accent.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            action_title,
                            ca_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            color,
                        );
                    }
                }

                if subsystems.has_git() && git_log_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let gl_w = (width * 0.6).max(500.0).min(width - 20.0);
                    let gl_x = (width - gl_w) / 2.0;
                    let gl_y = style.padding_y * 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_vis = 20usize;
                    let vis = git_log_entries.len().min(max_vis);
                    let gl_h = line_h * (vis as f64 + 1.0) + style.padding_y * 2.0;
                    draw_ctx.draw_rect(
                        gl_x - 1.0,
                        gl_y - 1.0,
                        gl_w + 2.0,
                        gl_h + 2.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rect(gl_x, gl_y, gl_w, gl_h, style.background3.to_array());
                    let input_y = gl_y + style.padding_y;
                    let title =
                        format!("Git Log  ({} commits)  [Esc] close", git_log_entries.len());
                    draw_ctx.draw_text(
                        style.font,
                        &title,
                        gl_x + style.padding_x,
                        input_y,
                        style.accent.to_array(),
                    );
                    draw_ctx.draw_rect(
                        gl_x,
                        input_y + line_h,
                        gl_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );
                    let scroll_off = if git_log_selected >= max_vis {
                        git_log_selected - max_vis + 1
                    } else {
                        0
                    };
                    for (i, (hash, date, msg)) in git_log_entries
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_vis)
                    {
                        let di = i - scroll_off;
                        let ry = input_y + line_h + style.divider_size + di as f64 * line_h;
                        if i == git_log_selected {
                            draw_ctx.draw_rect(gl_x, ry, gl_w, line_h, style.selection.to_array());
                        }
                        let entry_text = format!("{hash}  {date}  {msg}");
                        let hash_color = if i == git_log_selected {
                            style.accent.to_array()
                        } else {
                            style.dim.to_array()
                        };
                        draw_ctx.draw_text(
                            style.font,
                            &entry_text,
                            gl_x + style.padding_x,
                            ry + style.padding_y / 2.0,
                            hash_color,
                        );
                    }
                }

                // Draw command view (file/folder open with autocomplete) at top.
                if cmdview_active {
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    // Widen the picker to 70% of the window so common paths
                    // fit without scrolling. The input still hard-scrolls
                    // horizontally for anything longer, and the suggestions
                    // list ellipsis-truncates on the LEFT so the filename
                    // (the interesting part of a long path) stays visible.
                    let cv_w = (width * 0.7).max(500.0).min(width - 20.0);
                    let cv_x = (width - cv_w) / 2.0;
                    let line_h = style.font_height + style.padding_y;
                    let max_visible = 15usize;
                    let visible_count = cmdview_suggestions.len().min(max_visible);
                    let cv_h = line_h * (visible_count as f64 + 1.0) + style.padding_y * 2.0;
                    // When a nag is active, push the cmdview down so the
                    // nag bar stays visible at the top and its key focus
                    // isn't hidden behind the picker.
                    let nag_offset = if matches!(
                        nag,
                        Nag::OverwriteFile { .. }
                            | Nag::CreateDir { .. }
                            | Nag::ReloadFromDisk { .. }
                            | Nag::NoExtension { .. }
                    ) {
                        style.font_height + style.padding_y * 2.0 + style.padding_y
                    } else {
                        0.0
                    };
                    let cv_y = style.padding_y * 2.0 + nag_offset;

                    // Border + background.
                    let cv_r = 10.0;
                    draw_ctx.draw_rounded_rect(
                        cv_x - 1.0,
                        cv_y - 1.0,
                        cv_w + 2.0,
                        cv_h + 2.0,
                        cv_r,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rounded_rect(cv_x, cv_y, cv_w, cv_h, cv_r, style.background3.to_array());

                    // Input line.
                    let input_y = cv_y + style.padding_y;
                    let label = &cmdview_label;
                    let label_w = draw_ctx.font_width(style.font, label);
                    draw_ctx.draw_text(
                        style.font,
                        label,
                        cv_x + style.padding_x,
                        input_y,
                        style.accent.to_array(),
                    );

                    // Horizontal-scrolling input. `text_origin` is where the
                    // first character of the input would land if scroll == 0;
                    // we shift text left (via `text_scroll`) so the caret is
                    // always a few chars inside the visible area even for
                    // long paths. A tiny "<" / ">" indicator marks the edge
                    // when content exists past it so the user can tell
                    // they're scrolled.
                    let text_area_x = cv_x + style.padding_x + label_w + style.padding_x;
                    let text_area_right = cv_x + cv_w - style.padding_x;
                    let text_area_w = (text_area_right - text_area_x).max(0.0);
                    let cursor_safe = cmdview_cursor.min(cmdview_text.len());
                    let before_cursor = &cmdview_text[..cursor_safe];
                    let caret_offset_px = draw_ctx.font_width(style.font, before_cursor);
                    let full_text_w = draw_ctx.font_width(style.font, &cmdview_text);
                    let caret_margin = (style.font_height * 0.5).min(text_area_w * 0.25);
                    let mut text_scroll = if full_text_w <= text_area_w {
                        0.0
                    } else if caret_offset_px > text_area_w - caret_margin {
                        caret_offset_px - (text_area_w - caret_margin)
                    } else {
                        0.0
                    };
                    // Guarantee we don't scroll so far that we reveal blank
                    // space past the end of the text.
                    let max_scroll = (full_text_w - text_area_w).max(0.0);
                    if text_scroll > max_scroll {
                        text_scroll = max_scroll;
                    }
                    let text_origin = text_area_x - text_scroll;

                    // Clip text to the input area so long paths can't bleed
                    // over the label, the box border, or the scrollbar.
                    draw_ctx.set_clip_rect(text_area_x, input_y, text_area_w, style.font_height);
                    draw_ctx.draw_text(
                        style.font,
                        &cmdview_text,
                        text_origin,
                        input_y,
                        style.text.to_array(),
                    );
                    let caret_x = text_origin + caret_offset_px;
                    draw_ctx.draw_rect(
                        caret_x,
                        input_y,
                        style.caret_width,
                        style.font_height,
                        style.caret.to_array(),
                    );
                    draw_ctx.set_clip_rect(0.0, 0.0, width, height);
                    if text_scroll > 0.5 {
                        draw_ctx.draw_text(
                            style.font,
                            "<",
                            text_area_x - draw_ctx.font_width(style.font, "<"),
                            input_y,
                            style.dim.to_array(),
                        );
                    }
                    if full_text_w - text_scroll > text_area_w + 0.5 {
                        draw_ctx.draw_text(
                            style.font,
                            ">",
                            text_area_right,
                            input_y,
                            style.dim.to_array(),
                        );
                    }

                    // Divider below input.
                    draw_ctx.draw_rect(
                        cv_x,
                        input_y + line_h,
                        cv_w,
                        style.divider_size,
                        style.divider.to_array(),
                    );

                    // Scroll offset so selected item is visible.
                    let scroll_off = if cmdview_selected >= max_visible {
                        cmdview_selected - max_visible + 1
                    } else {
                        0
                    };

                    // Suggestions list. Long paths get ellipsis-truncated on
                    // the LEFT so the filename stays visible — that's
                    // typically what the user is trying to pick.
                    let suggestion_area_x = cv_x + style.padding_x;
                    let suggestion_area_w = (cv_w - style.padding_x * 2.0).max(0.0);
                    for (i, suggestion) in cmdview_suggestions
                        .iter()
                        .enumerate()
                        .skip(scroll_off)
                        .take(max_visible)
                    {
                        let display_idx = i - scroll_off;
                        let ry =
                            input_y + line_h + style.divider_size + display_idx as f64 * line_h;
                        if i == cmdview_selected {
                            let inner_pad = style.padding_x * 0.5;
                            draw_ctx.draw_rounded_rect(
                                cv_x + inner_pad,
                                ry + 1.0,
                                cv_w - inner_pad * 2.0,
                                line_h - 2.0,
                                6.0,
                                style.selection.to_array(),
                            );
                        }
                        let is_dir = suggestion.ends_with('/') || suggestion.ends_with('\\');
                        let color = if i == cmdview_selected || is_dir {
                            style.accent.to_array()
                        } else {
                            style.text.to_array()
                        };
                        let display_text = truncate_left_to_width(
                            suggestion,
                            suggestion_area_w,
                            style.font,
                            &mut draw_ctx,
                        );
                        draw_ctx.draw_text(
                            style.font,
                            &display_text,
                            suggestion_area_x,
                            ry + style.padding_y / 2.0,
                            color,
                        );
                    }
                }

                // Draw completion popup (LSP or document-word).
                if completion.visible && !completion.items.is_empty() {
                    if let Some(doc) = docs.get(active_tab) {
                        let dv = &doc.view;
                        crate::editor::app_state::clip_init(width, height);
                        use crate::editor::view::DrawContext as _;
                        let line_h_comp = style.line_height();
                        let gutter_w = dv.gutter_width;
                        let popup_x = dv.rect().x
                            + gutter_w
                            + style.padding_x
                            + (completion.col as f64 - 1.0)
                                * draw_ctx.font_width(style.code_font, "m")
                            - dv.scroll_x;
                        let item_h = style.font_height + style.padding_y;
                        // At most 10 items visible; the rest are reached
                        // by scrolling with Up/Down.
                        let max_visible = 10usize;
                        let visible_count = max_visible.min(completion.items.len());
                        let popup_h = item_h * visible_count as f64 + style.padding_y;
                        // Show just below the current line if there's room,
                        // otherwise flip above.
                        let cursor_screen_y =
                            dv.rect().y + completion.line as f64 * line_h_comp - dv.scroll_y;
                        let space_below = height - cursor_screen_y - line_h_comp;
                        let popup_y = if space_below >= popup_h {
                            cursor_screen_y + style.code_font_height + style.padding_y * 0.25
                        } else {
                            (cursor_screen_y - popup_h).max(0.0)
                        };
                        // Width = max label + detail over the visible
                        // items, clamped to screen edge and to a 120px min.
                        let content_w = completion
                            .items
                            .iter()
                            .skip(completion.scroll_offset)
                            .take(visible_count)
                            .map(|(label, detail, _)| {
                                let lw = draw_ctx.font_width(style.font, label);
                                if detail.is_empty() {
                                    lw
                                } else {
                                    lw + draw_ctx.font_width(style.font, detail) + style.padding_x
                                }
                            })
                            .fold(0.0_f64, f64::max);
                        let popup_w = (content_w + style.padding_x * 2.0)
                            .max(120.0)
                            .min(width - popup_x - 10.0);
                        // Stash the screen rect for mouse hit-testing.
                        completion.rect = (popup_x, popup_y, popup_w, popup_h);
                        // Background.
                        // Background + top border.
                        let popup_r = 10.0;
                        draw_ctx.draw_rounded_rect(
                            popup_x,
                            popup_y,
                            popup_w,
                            popup_h,
                            popup_r,
                            style.divider.to_array(),
                        );
                        draw_ctx.draw_rounded_rect(
                            popup_x,
                            popup_y + style.divider_size,
                            popup_w,
                            popup_h - style.divider_size,
                            popup_r,
                            style.background3.to_array(),
                        );
                        let inner_pad = style.padding_x * 0.5;
                        for vis_i in 0..visible_count {
                            let i = completion.scroll_offset + vis_i;
                            let iy = popup_y + style.padding_y / 2.0 + vis_i as f64 * item_h;
                            if i < completion.items.len() {
                                if i == completion.selected {
                                    draw_ctx.draw_rounded_rect(
                                        popup_x + inner_pad,
                                        iy + 1.0,
                                        popup_w - inner_pad * 2.0,
                                        item_h - 2.0,
                                        6.0,
                                        style.selection.to_array(),
                                    );
                                }
                                if let Some((label, detail, _)) = completion.items.get(i) {
                                    let fg = if i == completion.selected {
                                        style.accent.to_array()
                                    } else {
                                        style.text.to_array()
                                    };
                                    draw_ctx.draw_text(
                                        style.font,
                                        label,
                                        popup_x + style.padding_x,
                                        iy + style.padding_y / 2.0,
                                        fg,
                                    );
                                    if !detail.is_empty() {
                                        let label_w = draw_ctx.font_width(style.font, label);
                                        draw_ctx.draw_text(
                                            style.font,
                                            detail,
                                            popup_x + style.padding_x + label_w + style.padding_x,
                                            iy + style.padding_y / 2.0,
                                            style.dim.to_array(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Draw LSP hover tooltip.
                if subsystems.has_lsp()
                    && signature_help.visible
                    && !signature_help.text.is_empty()
                    && let Some(doc) = docs.get(active_tab)
                {
                    let dv = &doc.view;
                    crate::editor::app_state::clip_init(width, height);
                    use crate::editor::view::DrawContext as _;
                    let line_h_sig = style.line_height();
                    let gutter_w = dv.gutter_width;
                    let sig_x = dv.rect().x
                        + gutter_w
                        + style.padding_x
                        + (signature_help.col as f64 - 1.0)
                            * draw_ctx.font_width(style.code_font, "m")
                        - dv.scroll_x;
                    // Below the current line so the popup does not cover the call.
                    let sig_y = dv.rect().y + signature_help.line as f64 * line_h_sig - dv.scroll_y
                        + style.padding_y / 2.0;
                    let text: String = signature_help
                        .text
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(120)
                        .collect();
                    let w = draw_ctx.font_width(style.font, &text) + style.padding_x * 2.0;
                    let h = style.font_height + style.padding_y * 2.0;
                    draw_ctx.draw_rounded_rect(
                        sig_x - 1.0,
                        sig_y - 1.0,
                        w + 2.0,
                        h + 2.0,
                        10.0,
                        style.divider.to_array(),
                    );
                    draw_ctx.draw_rounded_rect(sig_x, sig_y, w, h, 10.0, style.background3.to_array());
                    draw_ctx.draw_text(
                        style.font,
                        &text,
                        sig_x + style.padding_x,
                        sig_y + style.padding_y,
                        style.accent.to_array(),
                    );
                }

                if subsystems.has_lsp() && hover.visible && !hover.text.is_empty() {
                    if let Some(doc) = docs.get(active_tab) {
                        let dv = &doc.view;
                        crate::editor::app_state::clip_init(width, height);
                        use crate::editor::view::DrawContext as _;
                        let line_h_hover = style.line_height();
                        let gutter_w = dv.gutter_width;
                        let hover_x = dv.rect().x
                            + gutter_w
                            + style.padding_x
                            + (hover.col as f64 - 1.0) * draw_ctx.font_width(style.code_font, "m")
                            - dv.scroll_x;
                        let hover_y = dv.rect().y + (hover.line as f64 - 1.0) * line_h_hover
                            - dv.scroll_y
                            - style.padding_y;
                        // Wrap text to lines for display.
                        let max_chars = 80;
                        let hover_lines: Vec<&str> = hover
                            .text
                            .lines()
                            .flat_map(|l| {
                                if l.len() <= max_chars {
                                    vec![l]
                                } else {
                                    l.as_bytes()
                                        .chunks(max_chars)
                                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                                        .collect()
                                }
                            })
                            .take(15)
                            .collect();
                        let line_count_h = hover_lines.len();
                        let tooltip_line_h = style.font_height + 2.0;
                        let tooltip_h =
                            tooltip_line_h * line_count_h as f64 + style.padding_y * 2.0;
                        let tooltip_w = hover_lines
                            .iter()
                            .map(|l| draw_ctx.font_width(style.font, l))
                            .fold(0.0_f64, f64::max)
                            + style.padding_x * 2.0;
                        let tooltip_y = hover_y - tooltip_h;
                        // Background + top border.
                        draw_ctx.draw_rounded_rect(
                            hover_x,
                            tooltip_y,
                            tooltip_w,
                            tooltip_h,
                            10.0,
                            style.divider.to_array(),
                        );
                        draw_ctx.draw_rounded_rect(
                            hover_x,
                            tooltip_y + style.divider_size,
                            tooltip_w,
                            tooltip_h - style.divider_size,
                            10.0,
                            style.background3.to_array(),
                        );
                        for (i, line_text) in hover_lines.iter().enumerate() {
                            draw_ctx.draw_text(
                                style.font,
                                line_text,
                                hover_x + style.padding_x,
                                tooltip_y + style.padding_y + i as f64 * tooltip_line_h,
                                style.text.to_array(),
                            );
                        }
                    }
                }

                // Tab-bar overlays (hover tooltip + overflow dropdown list)
                // render here so the breadcrumb / sidebar / doc view don't
                // paint over them. The tab bar draw pass captured `tab_hover`,
                // `tab_overlay_*`, and the per-tab rects; this pass consumes
                // them without recomputing widths.
                if tab_overlay_tbh > 0.0 {
                    use crate::editor::view::DrawContext as _;
                    crate::editor::app_state::clip_init(width, height);
                    let tbh = tab_overlay_tbh;

                    // Tooltip for a hovered (truncated) tab.
                    if let Some(hi) = tab_hover {
                        if tab_overlay_overflow && !tab_tooltip_suppressed {
                            if let (Some(doc), Some((tx_h, tw_h, _, full_label))) =
                                (docs.get(hi), tab_overlay_rects.get(hi))
                            {
                                let path = doc.path.clone();
                                let tip_font = style.font;
                                let name_w = draw_ctx.font_width(tip_font, full_label);
                                let max_tip_w =
                                    (width - sidebar_w - style.padding_x * 2.0).max(80.0);
                                let path_full_w = draw_ctx.font_width(tip_font, &path);
                                let (path_display, path_w) =
                                    if path_full_w + style.padding_x * 2.0 <= max_tip_w {
                                        (path.clone(), path_full_w)
                                    } else {
                                        // Front-ellipsize: keep the rightmost (most
                                        // specific) part of the path.
                                        let ell = "...";
                                        let ell_w = draw_ctx.font_width(tip_font, ell);
                                        let mut trimmed: String = path.clone();
                                        while trimmed.chars().count() > 1
                                            && ell_w
                                                + draw_ctx.font_width(tip_font, &trimmed)
                                                + style.padding_x * 2.0
                                                > max_tip_w
                                        {
                                            let mut ch = trimmed.chars();
                                            ch.next();
                                            trimmed = ch.as_str().to_string();
                                        }
                                        let out = format!("{ell}{trimmed}");
                                        let w = draw_ctx.font_width(tip_font, &out);
                                        (out, w)
                                    };
                                let tip_w = name_w.max(path_w) + style.padding_x * 2.0;
                                let tip_h = style.font_height * 2.0 + style.padding_y * 1.5;
                                let tip_x = (tx_h + tw_h / 2.0 - tip_w / 2.0)
                                    .max(sidebar_w)
                                    .min((width - tip_w).max(sidebar_w));
                                let tip_y = tbh + 2.0;
                                draw_ctx.draw_rounded_rect(
                                    tip_x - 1.0,
                                    tip_y - 1.0,
                                    tip_w + 2.0,
                                    tip_h + 2.0,
                                    10.0,
                                    style.divider.to_array(),
                                );
                                draw_ctx.draw_rounded_rect(
                                    tip_x,
                                    tip_y,
                                    tip_w,
                                    tip_h,
                                    10.0,
                                    style.background.to_array(),
                                );
                                draw_ctx.draw_text(
                                    tip_font,
                                    full_label,
                                    tip_x + style.padding_x,
                                    tip_y + style.padding_y * 0.5,
                                    style.text.to_array(),
                                );
                                draw_ctx.draw_text(
                                    tip_font,
                                    &path_display,
                                    tip_x + style.padding_x,
                                    tip_y + style.padding_y * 0.5 + style.font_height,
                                    style.dim.to_array(),
                                );
                            }
                        }
                    }

                    // Overflow dropdown list: right edge pinned to the dropdown
                    // button's right edge (= window right), extends leftward.
                    if tab_dropdown_open && tab_overlay_overflow {
                        let item_h = style.font_height + style.padding_y;
                        let mut list_w = 0.0_f64;
                        for doc in docs.iter() {
                            let label = if doc_is_modified(doc) {
                                format!("*{}", doc.name)
                            } else {
                                doc.name.clone()
                            };
                            list_w = list_w.max(
                                draw_ctx.font_width(style.font, &label) + style.padding_x * 3.0,
                            );
                        }
                        let avail_list_w = (width - sidebar_w - 4.0).max(40.0);
                        list_w = list_w.max(120.0).min(avail_list_w);
                        let btn_right = tab_overlay_btn_right;
                        let mut list_x = btn_right - list_w;
                        if list_x < sidebar_w + 2.0 {
                            list_x = sidebar_w + 2.0;
                        }
                        let max_list_h = (height - tbh - 4.0).max(item_h);
                        let raw_list_h = item_h * docs.len() as f64 + style.padding_y;
                        let list_h = raw_list_h.min(max_list_h);
                        let list_y = tbh;
                        draw_ctx.draw_rect(
                            list_x - 1.0,
                            list_y - 1.0,
                            list_w + 2.0,
                            list_h + 2.0,
                            style.divider.to_array(),
                        );
                        draw_ctx.draw_rect(
                            list_x,
                            list_y,
                            list_w,
                            list_h,
                            style.background.to_array(),
                        );
                        let mut iy = list_y + style.padding_y / 2.0;
                        for (i, doc) in docs.iter().enumerate() {
                            let label = if doc_is_modified(doc) {
                                format!("*{}", doc.name)
                            } else {
                                doc.name.clone()
                            };
                            let row_hover = mouse_x >= list_x
                                && mouse_x < list_x + list_w
                                && mouse_y >= iy
                                && mouse_y < iy + item_h;
                            if i == active_tab {
                                draw_ctx.draw_rect(
                                    list_x,
                                    iy,
                                    list_w,
                                    item_h,
                                    style.line_highlight.to_array(),
                                );
                            } else if row_hover {
                                draw_ctx.draw_rect(
                                    list_x,
                                    iy,
                                    list_w,
                                    item_h,
                                    style.selection.to_array(),
                                );
                            }
                            let color = if i == active_tab {
                                style.accent.to_array()
                            } else {
                                style.text.to_array()
                            };
                            draw_ctx.draw_text(
                                style.font,
                                &label,
                                list_x + style.padding_x,
                                iy + (item_h - style.font_height) / 2.0,
                                color,
                            );
                            iy += item_h;
                        }
                    }
                }
                let _ = tab_overlay_btn_w; // reserved for future hit-test overlays

                // Draw context menu on top of everything.
                if context_menu.visible {
                    crate::editor::app_state::clip_init(width, height);
                    context_menu.draw_native(&mut draw_ctx, &style, width, height);
                }

                crate::renderer::native_end_frame();

                // Keep redrawing while momentum scroll is still in motion.
                if config.transitions && !config.disabled_transitions.scroll {
                    if editor_scroll_vel.abs() > 0.5
                        || sidebar_scroll_vel.abs() > 0.5
                        || preview_scroll_vel.abs() > 0.5
                    {
                        redraw = true;
                    }
                }
                last_draw = Instant::now();
            }
        }
}
