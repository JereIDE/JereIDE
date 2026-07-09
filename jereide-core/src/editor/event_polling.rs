{
        while let Some(event) = crate::window::poll_event_native() {
            // Any event counts as activity for idle-drop tracking.
            had_input_events = true;
            last_activity = Instant::now();
            dropped_caches_for_idle = false;
            // Scope each dialog field's undo history to a single open session:
            // while the input is closed its history stays cleared, so reopening
            // starts fresh and Ctrl+Z can never restore text from a previous
            // session into an unrelated field.
            if !find_active {
                find_history.clear();
                replace_history.clear();
            }
            if !cmdview_active {
                cmdview_history.clear();
            }
            if !project_search_active {
                project_search_history.clear();
            }
            if !project_replace_active {
                project_replace_search_history.clear();
                project_replace_with_history.clear();
            }
            if !palette_active {
                palette_history.clear();
            }

            if sidebar_new_file_dir.is_none() {
                sidebar_new_file_history.clear();
            }
            match &event {
                EditorEvent::Quit => {
                    quit = true;
                }
                EditorEvent::Exposed | EditorEvent::Resized { .. } | EditorEvent::FocusGained => {
                    window_hidden = false;
                    redraw = true;
                }
                EditorEvent::Shown => {
                    window_hidden = false;
                    redraw = true;
                }
                EditorEvent::Occluded | EditorEvent::Hidden => {
                    window_hidden = true;
                    crate::renderer::drop_caches();
                }
                EditorEvent::KeyReleased { key, .. } => {
                    let k = key.as_str();
                    if k == "left shift" || k == "right shift" || k == "lshift" || k == "rshift" {
                        shift_held = false;
                    }
                    continue;
                }
                EditorEvent::KeyPressed { key, modifiers } => {
                    editor_scroll_vel = 0.0;
                    sidebar_scroll_vel = 0.0;
                    preview_scroll_vel = 0.0;
                    if let Some(doc) = docs.get_mut(active_tab) {
                        doc.view.scroll_y = doc.view.target_scroll_y;
                    }
                    // Modifier-only key presses (Ctrl/Shift/Alt/Gui alone) shouldn't
                    // touch the editor at all — no redraw, no blink reset, no scroll
                    // lerp tick. Only update the local shift tracker for shift+click.
                    // SDL reports modifier keys with platform-dependent names
                    // ("left ctrl" / "left control" / "lctrl"; "left gui" /
                    // "left meta" / "left super"), so match the family rather
                    // than a fixed string list.
                    let key_lc = key.as_str();
                    let is_modifier_only = matches!(
                        key_lc,
                        "left shift"
                            | "right shift"
                            | "lshift"
                            | "rshift"
                            | "left ctrl"
                            | "right ctrl"
                            | "lctrl"
                            | "rctrl"
                            | "left control"
                            | "right control"
                            | "left alt"
                            | "right alt"
                            | "lalt"
                            | "ralt"
                            | "left gui"
                            | "right gui"
                            | "lgui"
                            | "rgui"
                            | "left meta"
                            | "right meta"
                            | "lmeta"
                            | "rmeta"
                            | "left super"
                            | "right super"
                            | "lsuper"
                            | "rsuper"
                            | "left win"
                            | "right win"
                    );
                    if is_modifier_only {
                        if key_lc == "left shift"
                            || key_lc == "right shift"
                            || key_lc == "lshift"
                            || key_lc == "rshift"
                        {
                            shift_held = true;
                        }
                        continue;
                    }
                    cursor_blink_reset = Instant::now();
                    let mut mods = *modifiers;
                    // On macOS, optionally fold Cmd into Ctrl so Cmd+S acts
                    // like Ctrl+S. See NativeConfig::mac_command_as_ctrl.
                    if cfg!(target_os = "macos") && config.mac_command_as_ctrl && mods.gui {
                        mods.ctrl = true;
                        mods.gui = false;
                    }



                    // Tab overflow dropdown: Escape dismisses it.
                    if tab_dropdown_open && key.as_str() == "escape" {
                        tab_dropdown_open = false;
                        redraw = true;
                        continue;
                    }

                    // Context menu intercepts keys when visible.
                    if context_menu.visible {
                        match key.as_str() {
                            "escape" => {
                                context_menu.hide();
                                redraw = true;
                                continue;
                            }
                            "up" => {
                                if let Some(sel) = context_menu.selected {
                                    if sel > 0 {
                                        context_menu.selected = Some(sel - 1);
                                    }
                                } else if !context_menu.items.is_empty() {
                                    context_menu.selected = Some(context_menu.items.len() - 1);
                                }
                                redraw = true;
                                continue;
                            }
                            "down" => {
                                if let Some(sel) = context_menu.selected {
                                    if sel + 1 < context_menu.items.len() {
                                        context_menu.selected = Some(sel + 1);
                                    }
                                } else {
                                    context_menu.selected = Some(0);
                                }
                                redraw = true;
                                continue;
                            }
                            "return" | "keypad enter" => {
                                if let Some(sel) = context_menu.selected {
                                    if let Some(item) = context_menu.items.get(sel) {
                                        if let Some(ref cmd) = item.command {
                                            let cmd = cmd.clone();
                                            context_menu.hide();
                                            {
                                                include!("commands_dispatch.rs");
                                            }
                                        } else {
                                            context_menu.hide();
                                        }
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            _ => {
                                context_menu.hide();
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Completion popup intercepts keys when visible.
                    if completion.visible {
                        match key.as_str() {
                            "escape" => {
                                completion.hide();
                                redraw = true;
                                continue;
                            }
                            "up" => {
                                if completion.selected > 0 {
                                    completion.selected -= 1;
                                    // Scroll the window so the selected item stays visible.
                                    if completion.selected < completion.scroll_offset {
                                        completion.scroll_offset =
                                            completion.scroll_offset.saturating_sub(1);
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "down" => {
                                if completion.selected + 1 < completion.items.len() {
                                    completion.selected += 1;
                                    // Scroll the window so the selected item stays visible.
                                    let max_visible = 10usize;
                                    if completion.selected >= completion.scroll_offset + max_visible
                                    {
                                        completion.scroll_offset =
                                            completion.selected - max_visible + 1;
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "return" | "keypad enter" | "tab" => {
                                if let Some((_, _, insert_text)) =
                                    completion.items.get(completion.selected)
                                {
                                    let text = insert_text.clone();
                                    if let Some(doc) = docs.get_mut(active_tab) {
                                        if let Some(buf_id) = doc.view.buffer_id {
                                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                buffer::push_undo(b);
                                                let line = *b.selections.first().unwrap_or(&1);
                                                let col = *b.selections.get(1).unwrap_or(&1);
                                                if line <= b.lines.len() {
                                                    // Find the start of the word prefix
                                                    // at the cursor so we replace
                                                    // rather than append.
                                                    let l = &b.lines[line - 1];
                                                    let chars: Vec<char> = l.chars().collect();
                                                    let col_idx = (col - 1).min(chars.len());
                                                    let mut word_start = col_idx;
                                                    while word_start > 0 {
                                                        let c = chars[word_start - 1];
                                                        if c.is_alphanumeric() || c == '_' {
                                                            word_start -= 1;
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    let l = &mut b.lines[line - 1];
                                                    let byte_start = char_to_byte(l, word_start);
                                                    let byte_end = char_to_byte(l, col - 1);
                                                    l.replace_range(byte_start..byte_end, &text);
                                                    // word_start is 0-based; selections
                                                    // use 1-based columns.
                                                    let new_col =
                                                        word_start + 1 + text.chars().count();
                                                    b.selections[0] = line;
                                                    b.selections[1] = new_col;
                                                    b.selections[2] = line;
                                                    b.selections[3] = new_col;
                                                }
                                                Ok(())
                                            });
                                        }
                                    }
                                }
                                completion.hide();
                                redraw = true;
                                continue;
                            }
                            _ => {
                                completion.hide();
                                // Fall through to normal key handling.
                            }
                        }
                    }

                    // Dismiss hover on any keypress.
                    if hover.visible {
                        hover.hide();
                        redraw = true;
                    }
                    // Dismiss signature help on Escape; it persists while typing
                    // arguments so the parameter hint stays visible.
                    if key.as_str() == "escape" && signature_help.visible {
                        signature_help.hide();
                        redraw = true;
                    }

                    // Inline new-file input in the sidebar intercepts keys.
                    if sidebar_new_file_dir.is_some() && matches!(nag, Nag::None) {
                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            let restored = if is_redo {
                                sidebar_new_file_history
                                    .redo(&sidebar_new_file_name, sidebar_new_file_cursor)
                            } else {
                                sidebar_new_file_history
                                    .undo(&sidebar_new_file_name, sidebar_new_file_cursor)
                            };
                            if let Some((t, c)) = restored {
                                sidebar_new_file_name = t;
                                sidebar_new_file_cursor = c.min(sidebar_new_file_name.len());
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            match action {
                                FieldClipboard::Copy => {
                                    if !sidebar_new_file_name.is_empty() {
                                        crate::window::set_clipboard_text(&sidebar_new_file_name);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if !sidebar_new_file_name.is_empty() {
                                        crate::window::set_clipboard_text(&sidebar_new_file_name);
                                        sidebar_new_file_history.record(
                                            &sidebar_new_file_name,
                                            sidebar_new_file_cursor,
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        sidebar_new_file_name.clear();
                                        sidebar_new_file_cursor = 0;
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        sidebar_new_file_history.record(
                                            &sidebar_new_file_name,
                                            sidebar_new_file_cursor,
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        sidebar_new_file_cursor = insert_clipboard_line(
                                            &mut sidebar_new_file_name,
                                            sidebar_new_file_cursor,
                                            &clip,
                                        );
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        match key.as_str() {
                            "escape" => {
                                sidebar_new_file_dir = None;
                                sidebar_new_file_name.clear();
                                sidebar_new_file_cursor = 0;
                            }
                            "return" | "keypad enter" => {
                                let name = sidebar_new_file_name.trim().to_string();
                                let dir = sidebar_new_file_dir.take().unwrap_or_default();
                                sidebar_new_file_name.clear();
                                sidebar_new_file_cursor = 0;
                                if !name.is_empty() {
                                    let full_path = std::path::Path::new(&dir)
                                        .join(&name)
                                        .to_string_lossy()
                                        .to_string();
                                    if std::path::Path::new(&full_path).exists() {
                                        info_message = Some((
                                            format!("File already exists: {name}"),
                                            Instant::now(),
                                        ));
                                    } else {
                                        match std::fs::write(&full_path, "") {
                                            Ok(()) => {
                                                if subsystems.has_sidebar()
                                                    && !project_root.is_empty()
                                                {
                                                    // Snapshot in-memory expanded
                                                    // dirs so the rescan doesn't
                                                    // collapse the folder the user
                                                    // just created into.
                                                    let in_memory_expanded: HashSet<String> =
                                                        sidebar_entries
                                                            .iter()
                                                            .filter(|e| e.is_dir && e.expanded)
                                                            .map(|e| e.path.clone())
                                                            .collect();
                                                    sidebar_entries = scan_for_sidebar(
                                                                                                                &project_root,
                                                        sidebar_show_hidden,
                                                    );
                                                    restore_expanded_folders(
                                                        &mut sidebar_entries,
                                                        userdir_path,
                                                        sidebar_show_hidden,
                                                        &project_session_key(&project_root),
                                                    );
                                                    expand_sidebar_from_set(
                                                        &mut sidebar_entries,
                                                        &in_memory_expanded,
                                                        sidebar_show_hidden,
                                                    );
                                                }
                                                if open_file_into(&full_path, &mut docs, use_git())
                                                {
                                                    autoreload.watch(&full_path);
                                                    active_tab = docs.len() - 1;
                                                    remember_recent_file(
                                                        &mut recent_files,
                                                        &full_path,
                                                        userdir_path,
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                info_message = Some((
                                                    format!("Create failed: {e}"),
                                                    Instant::now(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            "backspace" if sidebar_new_file_cursor > 0 => {
                                sidebar_new_file_history.record(
                                    &sidebar_new_file_name,
                                    sidebar_new_file_cursor,
                                    FieldEdit::Delete,
                                    buffer::now_secs(),
                                );
                                let prev = sidebar_new_file_name[..sidebar_new_file_cursor]
                                    .char_indices()
                                    .next_back()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                sidebar_new_file_name.drain(prev..sidebar_new_file_cursor);
                                sidebar_new_file_cursor = prev;
                            }
                            "backspace" => {}
                            "delete" if sidebar_new_file_cursor < sidebar_new_file_name.len() => {
                                sidebar_new_file_history.record(
                                    &sidebar_new_file_name,
                                    sidebar_new_file_cursor,
                                    FieldEdit::Delete,
                                    buffer::now_secs(),
                                );
                                let next = sidebar_new_file_name[sidebar_new_file_cursor..]
                                    .char_indices()
                                    .nth(1)
                                    .map(|(i, _)| sidebar_new_file_cursor + i)
                                    .unwrap_or(sidebar_new_file_name.len());
                                sidebar_new_file_name.drain(sidebar_new_file_cursor..next);
                            }
                            "delete" => {}
                            "left" if sidebar_new_file_cursor > 0 => {
                                sidebar_new_file_cursor = sidebar_new_file_name
                                    [..sidebar_new_file_cursor]
                                    .char_indices()
                                    .next_back()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                            }
                            "left" => {}
                            "right" if sidebar_new_file_cursor < sidebar_new_file_name.len() => {
                                sidebar_new_file_cursor = sidebar_new_file_name
                                    [sidebar_new_file_cursor..]
                                    .char_indices()
                                    .nth(1)
                                    .map(|(i, _)| sidebar_new_file_cursor + i)
                                    .unwrap_or(sidebar_new_file_name.len());
                            }
                            "right" => {}
                            "home" => {
                                sidebar_new_file_cursor = 0;
                            }
                            "end" => {
                                sidebar_new_file_cursor = sidebar_new_file_name.len();
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Command view (file/folder open) intercepts keys — but
                    // only while no nag is active. When a modal nag (overwrite,
                    // create-dir, reload-from-disk) is up the cmdview stays on
                    // screen but its keypress arm must step aside so Y / N /
                    // Enter can reach the nag handler below.
                    if cmdview_active
                        && matches!(nag, Nag::None)
                        && (subsystems.has_picker()
                            || cmdview_mode == CmdViewMode::SaveAs
                            || cmdview_mode == CmdViewMode::OpenFile
                            || cmdview_mode == CmdViewMode::OpenRecent
                            || cmdview_mode == CmdViewMode::Rename)
                    {
                        /// Expand ~ and resolve relative paths to absolute.
                        /// On Windows, treat both `/` and `\` as absolute-path
                        /// indicators (`C:\...`) and use `USERPROFILE` for `~`.
                        fn expand_path(text: &str, project_root: &str) -> String {
                            let home_key = if cfg!(target_os = "windows") {
                                "USERPROFILE"
                            } else {
                                "HOME"
                            };
                            if let Some(rest) = text.strip_prefix('~') {
                                if let Some(home) = std::env::var_os(home_key) {
                                    return format!("{}{rest}", home.to_string_lossy());
                                }
                            }
                            if std::path::Path::new(text).is_absolute() {
                                return text.to_string();
                            }
                            let joined = std::path::Path::new(project_root)
                                .join(text)
                                .to_string_lossy()
                                .into_owned();
                            normalize_path(&joined)
                        }

                        /// Byte index of the previous character before `cursor` in `text`.
                        fn cmdview_prev_char(text: &str, cursor: usize) -> usize {
                            text[..cursor]
                                .char_indices()
                                .next_back()
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                        /// Byte index of the next character at or after `cursor` in `text`.
                        fn cmdview_next_char(text: &str, cursor: usize) -> usize {
                            if cursor >= text.len() {
                                return text.len();
                            }
                            text[cursor..]
                                .char_indices()
                                .nth(1)
                                .map(|(i, _)| cursor + i)
                                .unwrap_or(text.len())
                        }
                        /// Jump left to the start of the previous path segment.
                        /// Accepts both `/` and `\` as separators so Windows
                        /// paths with backslashes behave the same as Unix
                        /// forward-slash paths.
                        fn cmdview_word_left(text: &str, cursor: usize) -> usize {
                            if cursor == 0 {
                                return 0;
                            }
                            let s = &text[..cursor];
                            let stripped = s.trim_end_matches(['/', '\\']);
                            if let Some(idx) = stripped.rfind(['/', '\\']) {
                                idx + 1
                            } else {
                                0
                            }
                        }
                        /// Jump right to the start of the next path segment.
                        fn cmdview_word_right(text: &str, cursor: usize) -> usize {
                            if cursor >= text.len() {
                                return text.len();
                            }
                            let rest = &text[cursor..];
                            let skip = if rest.starts_with('/') || rest.starts_with('\\') {
                                1
                            } else {
                                0
                            };
                            match rest[skip..].find(['/', '\\']) {
                                Some(idx) => cursor + skip + idx + 1,
                                None => text.len(),
                            }
                        }

                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            // Route the undo/redo bindings to the picker input.
                            let restored = if is_redo {
                                cmdview_history.redo(&cmdview_text, cmdview_cursor)
                            } else {
                                cmdview_history.undo(&cmdview_text, cmdview_cursor)
                            };
                            if let Some((t, c)) = restored {
                                cmdview_text = t;
                                cmdview_cursor = c.min(cmdview_text.len());
                                refresh_cmdview_suggestions(
                                    cmdview_mode,
                                    &cmdview_text,
                                    &project_root,
                                    &recent_files,
                                    &recent_projects,
                                    !false,
                                    &mut cmdview_suggestions,
                                );
                                cmdview_selected = 0;
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            match action {
                                FieldClipboard::Copy => {
                                    if !cmdview_text.is_empty() {
                                        crate::window::set_clipboard_text(&cmdview_text);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if !cmdview_text.is_empty() {
                                        crate::window::set_clipboard_text(&cmdview_text);
                                        cmdview_history.record(
                                            &cmdview_text,
                                            cmdview_cursor,
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        cmdview_text.clear();
                                        cmdview_cursor = 0;
                                        refresh_cmdview_suggestions(
                                            cmdview_mode,
                                            &cmdview_text,
                                            &project_root,
                                            &recent_files,
                                            &recent_projects,
                                            !false,
                                            &mut cmdview_suggestions,
                                        );
                                        cmdview_selected = 0;
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        cmdview_history.record(
                                            &cmdview_text,
                                            cmdview_cursor,
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        cmdview_cursor = insert_clipboard_line(
                                            &mut cmdview_text,
                                            cmdview_cursor,
                                            &clip,
                                        );
                                        refresh_cmdview_suggestions(
                                            cmdview_mode,
                                            &cmdview_text,
                                            &project_root,
                                            &recent_files,
                                            &recent_projects,
                                            !false,
                                            &mut cmdview_suggestions,
                                        );
                                        cmdview_selected = 0;
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        match key.as_str() {
                            "escape" => {
                                cmdview_active = false;
                            }
                            "return" | "keypad enter" => {
                                // Go-to-line mode: parse number and jump.
                                if cmdview_label.starts_with("Go To Line") {
                                    if let Ok(target) = cmdview_text.trim().parse::<usize>() {
                                        if let Some(doc) = docs.get_mut(active_tab) {
                                            if let Some(buf_id) = doc.view.buffer_id {
                                                let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                    let ln = target.clamp(1, b.lines.len());
                                                    b.selections = vec![ln, 1, ln, 1];
                                                    Ok(())
                                                });
                                                let line_h = style.line_height();
                                                doc.view.scroll_y = ((target as f64 - 1.0)
                                                    * line_h
                                                    - doc.view.rect().h / 2.0)
                                                    .max(0.0);
                                                doc.view.target_scroll_y = doc.view.scroll_y;
                                            }
                                        }
                                    }
                                    cmdview_active = false;
                                    redraw = true;
                                    continue;
                                }
                                // In Save As, Enter commits exactly what the user
                                // typed — never the highlighted suggestion — so
                                // autocomplete races can't silently retarget the
                                // save onto an existing file. Other modes keep
                                // the old "use suggestion if one is highlighted"
                                // behaviour so Enter on a sidebar match still
                                // works.
                                let chosen = if cmdview_mode == CmdViewMode::SaveAs {
                                    cmdview_text.clone()
                                } else if !cmdview_suggestions.is_empty()
                                    && cmdview_selected < cmdview_suggestions.len()
                                {
                                    cmdview_suggestions[cmdview_selected].clone()
                                } else {
                                    cmdview_text.clone()
                                };
                                let path = expand_path(&chosen, &project_root);
                                let path = path.trim_end_matches('/').to_string();
                                let p = std::path::Path::new(&path);
                                match cmdview_mode {
                                    CmdViewMode::LspRename => {
                                        // The typed text is the new symbol name, not
                                        // a path, so use it directly.
                                        let new_name = cmdview_text.trim().to_string();
                                        if !new_name.is_empty()
                                            && let Some((uri, line0, char0)) = lsp_rename_pos.take()
                                            && subsystems.has_lsp()
                                            && lsp_state.initialized
                                            && let Some(tid) = lsp_state.transport_id
                                        {
                                            let req_id = lsp_state.next_id();
                                            lsp_state
                                                .pending_requests
                                                .insert(req_id, "textDocument/rename".to_string());
                                            let _ = lsp::send_message(
                                                tid,
                                                &lsp_rename_request(
                                                    req_id, &uri, line0, char0, &new_name,
                                                ),
                                            );
                                        }
                                        lsp_rename_pos = None;
                                        cmdview_active = false;
                                        redraw = true;
                                        continue;
                                    }
                                    CmdViewMode::OpenFile => {
                                        // Support path:N to open at a specific line.
                                        let (file_path, goto_line) = split_path_line(&path);
                                        let (actual, line) = if goto_line.is_some()
                                            && !p.is_file()
                                            && std::path::Path::new(file_path).is_file()
                                        {
                                            (file_path.to_string(), goto_line)
                                        } else {
                                            (path.clone(), None)
                                        };
                                        let ap = std::path::Path::new(&actual);
                                        if ap.is_file() {
                                            cmdview_active = false;
                                            if false {
                                                // Replace current doc.
                                                for d in &docs { autoreload.unwatch(&d.path); }
                                                docs.clear();
                                                active_tab = 0;
                                            }
                                            match check_file_size_limit(
                                                &actual,
                                                config.large_file.hard_limit_mb,
                                            ) {
                                                Err(msg) => {
                                                    info_message = Some((msg, Instant::now()));
                                                }
                                                Ok(sz) => {
                                                    if sz > BG_LOAD_THRESHOLD && load_job.is_none() {
                                                        load_job = Some(spawn_load(&actual, sz));
                                                    } else if open_file_into(&actual, &mut docs, use_git()) {
                                                        active_tab = docs.len() - 1;
                                                        autoreload.watch(&actual);
                                                        remember_recent_file(&mut recent_files, &actual, userdir_path);
                                                        if let Some(ln) = line {
                                                            scroll_new_doc_to_line(
                                                                &mut docs,
                                                                ln,
                                                                style.line_height(),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        } else if ap.is_dir() {
                                            // Navigate into directory.
                                            cmdview_history.record(
                                                &cmdview_text,
                                                cmdview_cursor,
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            cmdview_text = dir_with_trailing_sep(&path);
                                            cmdview_cursor = cmdview_text.len();
                                            cmdview_suggestions =
                                                path_suggest(&cmdview_text, &project_root, false);
                                            cmdview_selected = 0;
                                        }
                                    }
                                    CmdViewMode::OpenFolder => {
                                        if p.is_dir() {
                                            // Check for unsaved changes before switching.
                                            if docs.iter().any(doc_is_modified) {
                                                nag = Nag::UnsavedChanges { message: nag_msg_quit(&docs), tab_to_close: None };
                                            } else {
                                                if subsystems.has_sidebar() {
                                                    save_project_session(
                                                        userdir_path,
                                                        &project_root,
                                                        &docs,
                                                        active_tab,
                                                    );
                                                    save_expanded_folders(&sidebar_entries, userdir_path, &project_session_key(&project_root));
                                                }
                                                for d in &docs {
                                                    autoreload.unwatch(&d.path);
                                                }
                                                docs.clear();
                                                active_tab = 0;
                                                cmdview_active = false;
                                                project_root = path;
                                                if subsystems.has_sidebar() {
                                                    sidebar_watcher.unwatch_all();
                                                    sidebar_entries = scan_for_sidebar(
                                                                                                                &project_root,
                                                        sidebar_show_hidden,
                                                    );
                                                    restore_expanded_folders(
                                                        &mut sidebar_entries,
                                                        userdir_path,
                                                        sidebar_show_hidden,
                                                        &project_session_key(&project_root),
                                                    );
                                                    sidebar_watcher.watch_dir(&project_root);
                                                    for entry in &sidebar_entries {
                                                        if entry.is_dir && entry.expanded {
                                                            sidebar_watcher
                                                                .watch_dir(&entry.path);
                                                        }
                                                    }
                                                    sidebar_visible = true;
                                                    if let Some(tab) = restore_project_session(
                                                        userdir_path,
                                                        &project_root,
                                                        &mut docs,
                                                        &mut autoreload, use_git(),
                                                    ) {
                                                        active_tab = tab;
                                                    }
                                                }
                                                let abs = std::fs::canonicalize(&project_root)
                                                    .map(|p| p.to_string_lossy().to_string())
                                                    .unwrap_or_else(|_| project_root.clone());
                                                recent_projects.retain(|p| p != &abs);
                                                recent_projects.insert(0, abs);
                                                if recent_projects.len() > 20 {
                                                    recent_projects.truncate(20);
                                                }
                                                let _ = crate::editor::storage::save_text(
                                                    userdir_path,
                                                    "session",
                                                    "recent_projects",
                                                    &serde_json::to_string(&recent_projects)
                                                        .unwrap_or_default(),
                                                );
                                            }
                                        }
                                    }
                                    CmdViewMode::OpenRecent => {
                                        cmdview_active = false;
                                        if p.is_file() {
                                            if open_file_into(&path, &mut docs, use_git()) {
                                                active_tab = docs.len() - 1;
                                                autoreload.watch(&path);
                                                remember_recent_file(&mut recent_files, &path, userdir_path);
                                            }
                                        } else if p.is_dir() {
                                            if docs.iter().any(doc_is_modified) {
                                                nag = Nag::UnsavedChanges { message: nag_msg_quit(&docs), tab_to_close: None };
                                            } else {
                                                if subsystems.has_sidebar() {
                                                    save_project_session(
                                                        userdir_path,
                                                        &project_root,
                                                        &docs,
                                                        active_tab,
                                                    );
                                                    save_expanded_folders(&sidebar_entries, userdir_path, &project_session_key(&project_root));
                                                }
                                                for d in &docs {
                                                    autoreload.unwatch(&d.path);
                                                }
                                                docs.clear();
                                                active_tab = 0;
                                                project_root = path;
                                                if subsystems.has_sidebar() {
                                                    sidebar_watcher.unwatch_all();
                                                    sidebar_entries = scan_for_sidebar(
                                                                                                                &project_root,
                                                        sidebar_show_hidden,
                                                    );
                                                    restore_expanded_folders(
                                                        &mut sidebar_entries,
                                                        userdir_path,
                                                        sidebar_show_hidden,
                                                        &project_session_key(&project_root),
                                                    );
                                                    sidebar_watcher.watch_dir(&project_root);
                                                    for entry in &sidebar_entries {
                                                        if entry.is_dir && entry.expanded {
                                                            sidebar_watcher
                                                                .watch_dir(&entry.path);
                                                        }
                                                    }
                                                    sidebar_visible = true;
                                                    if let Some(tab) = restore_project_session(
                                                        userdir_path,
                                                        &project_root,
                                                        &mut docs,
                                                        &mut autoreload, use_git(),
                                                    ) {
                                                        active_tab = tab;
                                                    }
                                                }
                                                update_recent(
                                                    &mut recent_projects,
                                                    &project_root,
                                                    20,
                                                );
                                                let _ = crate::editor::storage::save_text(
                                                    userdir_path,
                                                    "session",
                                                    "recent_projects",
                                                    &serde_json::to_string(&recent_projects)
                                                        .unwrap_or_default(),
                                                );
                                            }
                                        }
                                    }
                                    CmdViewMode::SaveAs => {
                                        // Save current document to the chosen path.
                                        let save_path = if p.is_dir() {
                                            // User selected a directory -- stay in cmdview.
                                            cmdview_history.record(
                                                &cmdview_text,
                                                cmdview_cursor,
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            cmdview_text = dir_with_trailing_sep(&path);
                                            cmdview_cursor = cmdview_text.len();
                                            cmdview_suggestions = path_suggest(&cmdview_text, &project_root, false);
                                            cmdview_selected = 0;
                                            continue;
                                        } else {
                                            path.clone()
                                        };
                                        // If the parent directory is missing,
                                        // defer the save until the user confirms
                                        // creating the missing directories.
                                        let parent_missing = std::path::Path::new(&save_path)
                                            .parent()
                                            .map(|p| {
                                                !p.as_os_str().is_empty() && !p.exists()
                                            })
                                            .unwrap_or(false);
                                        if parent_missing {
                                            let parent_str = std::path::Path::new(&save_path)
                                                .parent()
                                                .map(|p| p.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            nag = Nag::CreateDir { parent: parent_str, save_path: save_path.clone(), doc_tab: active_tab, from_save_as: true };
                                            continue;
                                        }
                                        // Warn if the target filename has no
                                        // extension — common typo / forgot-to-
                                        // type-.ext case. Check the last path
                                        // segment so `/etc/hosts` (no ext) still
                                        // nags, and `foo.bar/README` counts the
                                        // filename as having no ext.
                                        let fname = std::path::Path::new(&save_path)
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("");
                                        let has_ext = fname
                                            .rfind('.')
                                            .is_some_and(|i| i > 0 && i < fname.len() - 1);
                                        if !has_ext {
                                            nag = Nag::NoExtension {
                                                save_path: save_path.clone(),
                                                doc_tab: active_tab,
                                            };
                                            redraw = true;
                                            continue;
                                        }
                                        // If the target exists and isn't the
                                        // current doc's own path, nag for
                                        // overwrite confirmation. This blocks
                                        // the autocomplete-races-Enter case
                                        // where a late-arriving suggestion
                                        // silently retargets the save.
                                        let own_path = docs
                                            .get(active_tab)
                                            .map(|d| d.path.as_str())
                                            .unwrap_or("");
                                        if std::path::Path::new(&save_path).is_file()
                                            && save_path != own_path
                                        {
                                            nag = Nag::OverwriteFile {
                                                save_path: save_path.clone(),
                                                doc_tab: active_tab,
                                            };
                                            redraw = true;
                                            continue;
                                        }
                                        if let Some(doc) = docs.get_mut(active_tab) {
                                            if let Some(buf_id) = doc.view.buffer_id {
                                                let atomic = config.files.atomic_save;
                                                let saved_id = buffer::with_buffer(buf_id, |b| {
                                                    buffer::save_file(b, &save_path, b.crlf, atomic)
                                                        .map_err(|_| buffer::BufferError::UnknownBuffer)?;
                                                    Ok(b.change_id)
                                                });
                                                if let Ok(id) = saved_id {
                                                    doc.saved_change_id = id;
                                                    doc.saved_signature = buffer::with_buffer(buf_id, |b| Ok(buffer::content_signature(&b.lines))).unwrap_or(0);
                                                    doc.path = save_path.clone();
                                                    doc.name = std::path::Path::new(&save_path)
                                                        .file_name()
                                                        .map(|n| n.to_string_lossy().to_string())
                                                        .unwrap_or_else(|| save_path.clone());
                                                    doc.cached_change_id = -1;
                                                    doc.cached_render = std::sync::Arc::new(Vec::new());
                                                    autoreload.watch(&save_path);
                                                    log_to_file(userdir, &format!("Saved {save_path}"));
                                                    info_message = Some((format!("Saved {}", doc.name), Instant::now()));
                                                } else {
                                                    info_message = Some((format!("Failed to save {save_path}"), Instant::now()));
                                                }
                                            }
                                        }
                                        // Save-as can create a new file or land an existing
                                        // buffer at a fresh path — rescan so the sidebar
                                        // picks it up. Gated on project_root prefix so
                                        // saves outside the project don't trigger a scan.
                                        if subsystems.has_sidebar()
                                            && !project_root.is_empty()
                                            && std::path::Path::new(&save_path)
                                                .starts_with(std::path::Path::new(&project_root))
                                        {
                                            sidebar_entries = scan_for_sidebar(
                                                                                                &project_root,
                                                sidebar_show_hidden,
                                            );
                                            restore_expanded_folders(
                                                &mut sidebar_entries,
                                                userdir_path,
                                                sidebar_show_hidden,
                                                &project_session_key(&project_root),
                                            );
                                        }
                                        cmdview_active = false;
                                    }
                                    CmdViewMode::Rename => {
                                        let src = std::mem::take(&mut rename_source);
                                        let dst = path.clone();
                                        cmdview_active = false;
                                        if src.is_empty() || src == dst {
                                            // nothing to do
                                        } else if std::path::Path::new(&dst).exists() {
                                            info_message = Some((
                                                format!("Target exists: {dst}"),
                                                Instant::now(),
                                            ));
                                        } else {
                                            if let Some(parent) =
                                                std::path::Path::new(&dst).parent()
                                            {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            match std::fs::rename(&src, &dst) {
                                                Ok(()) => {
                                                    for d in docs.iter_mut() {
                                                        if d.path == src {
                                                            autoreload.unwatch(&src);
                                                            d.path = dst.clone();
                                                            d.name = std::path::Path::new(&dst)
                                                                .file_name()
                                                                .map(|n| {
                                                                    n.to_string_lossy().to_string()
                                                                })
                                                                .unwrap_or_else(|| dst.clone());
                                                            autoreload.watch(&dst);
                                                        }
                                                    }
                                                    if subsystems.has_sidebar()
                                                        && !project_root.is_empty()
                                                    {
                                                        sidebar_entries = scan_for_sidebar(
                                                                                                                        &project_root,
                                                            sidebar_show_hidden,
                                                        );
                                                        restore_expanded_folders(
                                                            &mut sidebar_entries,
                                                            userdir_path,
                                                            sidebar_show_hidden,
                                                            &project_session_key(&project_root),
                                                        );
                                                    }
                                                    info_message = Some((
                                                        format!("Renamed to {dst}"),
                                                        Instant::now(),
                                                    ));
                                                }
                                                Err(e) => {
                                                    info_message = Some((
                                                        format!("Rename failed: {e}"),
                                                        Instant::now(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "tab"
                                // Select current suggestion: replace text, refresh.
                                if !cmdview_suggestions.is_empty()
                                    && cmdview_selected < cmdview_suggestions.len()
                                => {
                                    cmdview_history.record(
                                        &cmdview_text,
                                        cmdview_cursor,
                                        FieldEdit::Replace,
                                        buffer::now_secs(),
                                    );
                                    cmdview_text = cmdview_suggestions[cmdview_selected].clone();
                                    cmdview_cursor = cmdview_text.len();
                                    let dirs_only = cmdview_mode == CmdViewMode::OpenFolder;
                                    cmdview_suggestions =
                                        path_suggest(&cmdview_text, &project_root, dirs_only);
                                    cmdview_selected = 0;
                                }
                            "up" => {
                                if cmdview_selected > 0 {
                                    cmdview_selected -= 1;
                                } else if !cmdview_suggestions.is_empty() {
                                    cmdview_selected = cmdview_suggestions.len() - 1;
                                }
                            }
                            "down"
                                if !cmdview_suggestions.is_empty() => {
                                    cmdview_selected =
                                        (cmdview_selected + 1) % cmdview_suggestions.len();
                                }
                            "left" => {
                                if mods.ctrl {
                                    cmdview_cursor =
                                        cmdview_word_left(&cmdview_text, cmdview_cursor);
                                } else {
                                    cmdview_cursor =
                                        cmdview_prev_char(&cmdview_text, cmdview_cursor);
                                }
                            }
                            "right" => {
                                if mods.ctrl {
                                    cmdview_cursor =
                                        cmdview_word_right(&cmdview_text, cmdview_cursor);
                                } else if cmdview_cursor == cmdview_text.len()
                                    && !cmdview_suggestions.is_empty()
                                    && cmdview_selected < cmdview_suggestions.len()
                                {
                                    // Right-arrow at end of input accepts the
                                    // highlighted suggestion (like Tab) so
                                    // users aren't forced to press Enter —
                                    // which also commits the action and can
                                    // race a late autocomplete update.
                                    cmdview_history.record(
                                        &cmdview_text,
                                        cmdview_cursor,
                                        FieldEdit::Replace,
                                        buffer::now_secs(),
                                    );
                                    cmdview_text =
                                        cmdview_suggestions[cmdview_selected].clone();
                                    cmdview_cursor = cmdview_text.len();
                                    let dirs_only = cmdview_mode == CmdViewMode::OpenFolder;
                                    cmdview_suggestions = path_suggest(
                                        &cmdview_text,
                                        &project_root,
                                        dirs_only,
                                    );
                                    cmdview_selected = 0;
                                } else {
                                    cmdview_cursor =
                                        cmdview_next_char(&cmdview_text, cmdview_cursor);
                                }
                            }
                            "home" => {
                                cmdview_cursor = 0;
                            }
                            "end" => {
                                cmdview_cursor = cmdview_text.len();
                            }
                            "delete"
                                if cmdview_cursor < cmdview_text.len() => {
                                    cmdview_history.record(
                                        &cmdview_text,
                                        cmdview_cursor,
                                        FieldEdit::Delete,
                                        buffer::now_secs(),
                                    );
                                    let next = cmdview_next_char(&cmdview_text, cmdview_cursor);
                                    cmdview_text.replace_range(cmdview_cursor..next, "");
                                    refresh_cmdview_suggestions(
                                        cmdview_mode,
                                        &cmdview_text,
                                        &project_root,
                                        &recent_files,
                                        &recent_projects,
                                        !false,
                                        &mut cmdview_suggestions,
                                    );
                                    cmdview_selected = 0;
                                }
                            "backspace" => {
                                if cmdview_cursor > 0 {
                                    cmdview_history.record(
                                        &cmdview_text,
                                        cmdview_cursor,
                                        FieldEdit::Delete,
                                        buffer::now_secs(),
                                    );
                                }
                                if mods.ctrl {
                                    // Delete the previous path segment up to the cursor.
                                    let segment_start =
                                        cmdview_word_left(&cmdview_text, cmdview_cursor);
                                    cmdview_text.replace_range(segment_start..cmdview_cursor, "");
                                    cmdview_cursor = segment_start;
                                } else if cmdview_cursor > 0 {
                                    let prev = cmdview_prev_char(&cmdview_text, cmdview_cursor);
                                    cmdview_text.replace_range(prev..cmdview_cursor, "");
                                    cmdview_cursor = prev;
                                }
                                refresh_cmdview_suggestions(
                                    cmdview_mode,
                                    &cmdview_text,
                                    &project_root,
                                    &recent_files,
                                    &recent_projects,
                                    !false,
                                    &mut cmdview_suggestions,
                                );
                                cmdview_selected = 0;
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Project search intercepts keys when active.
                    if subsystems.has_find_in_files() && project_search_active {
                        if mods.alt && !mods.ctrl {
                            let toggled = match key.as_str() {
                                "r" => {
                                    project_use_regex = !project_use_regex;
                                    true
                                }
                                "w" => {
                                    project_whole_word = !project_whole_word;
                                    true
                                }
                                "i" => {
                                    project_case_insensitive = !project_case_insensitive;
                                    true
                                }
                                _ => false,
                            };
                            if toggled {
                                project_search_results = project_search::run_project_search(
                                    &project_search_query,
                                    &project_root,
                                    project_use_regex,
                                    project_whole_word,
                                    project_case_insensitive,
                                );
                                project_search_selected = 0;
                                redraw = true;
                                continue;
                            }
                        }
                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            let restored = if is_redo {
                                project_search_history
                                    .redo(&project_search_query, project_search_query.len())
                            } else {
                                project_search_history
                                    .undo(&project_search_query, project_search_query.len())
                            };
                            if let Some((t, _)) = restored {
                                project_search_query = t;
                                project_search_results = project_search::run_project_search(
                                    &project_search_query,
                                    &project_root,
                                    project_use_regex,
                                    project_whole_word,
                                    project_case_insensitive,
                                );
                                project_search_selected = 0;
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            match action {
                                FieldClipboard::Copy => {
                                    if !project_search_query.is_empty() {
                                        crate::window::set_clipboard_text(&project_search_query);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if !project_search_query.is_empty() {
                                        crate::window::set_clipboard_text(&project_search_query);
                                        project_search_history.record(
                                            &project_search_query,
                                            project_search_query.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        project_search_query.clear();
                                        project_search_results = project_search::run_project_search(
                                            &project_search_query,
                                            &project_root,
                                            project_use_regex,
                                            project_whole_word,
                                            project_case_insensitive,
                                        );
                                        project_search_selected = 0;
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        project_search_history.record(
                                            &project_search_query,
                                            project_search_query.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        append_clipboard_line(&mut project_search_query, &clip);
                                        project_search_results = project_search::run_project_search(
                                            &project_search_query,
                                            &project_root,
                                            project_use_regex,
                                            project_whole_word,
                                            project_case_insensitive,
                                        );
                                        project_search_selected = 0;
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        match key.as_str() {
                            "escape" => {
                                project_search_active = false;
                            }
                            "return" | "keypad enter" => {
                                if let Some((path, line_num, _)) =
                                    project_search_results.get(project_search_selected).cloned()
                                {
                                    project_search_active = false;
                                    // Open or switch to the file.
                                    let tab_idx = docs.iter().position(|d| d.path == path);
                                    let idx = if let Some(i) = tab_idx {
                                        i
                                    } else if open_file_into(&path, &mut docs, use_git()) {
                                        autoreload.watch(&path);
                                        remember_recent_file(
                                            &mut recent_files,
                                            &path,
                                            userdir_path,
                                        );
                                        docs.len() - 1
                                    } else {
                                        redraw = true;
                                        continue;
                                    };
                                    active_tab = idx;
                                    // Move cursor to the matched line.
                                    if let Some(doc) = docs.get_mut(active_tab) {
                                        if let Some(buf_id) = doc.view.buffer_id {
                                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                let target = line_num.min(b.lines.len()).max(1);
                                                b.selections[0] = target;
                                                b.selections[1] = 1;
                                                b.selections[2] = target;
                                                b.selections[3] = 1;
                                                Ok(())
                                            });
                                        }
                                    }
                                }
                            }
                            "up" => {
                                project_search_selected = project_search_selected.saturating_sub(1);
                            }
                            "down" if !project_search_results.is_empty() => {
                                project_search_selected = (project_search_selected + 1)
                                    .min(project_search_results.len() - 1);
                            }
                            "backspace" => {
                                if !project_search_query.is_empty() {
                                    project_search_history.record(
                                        &project_search_query,
                                        project_search_query.len(),
                                        FieldEdit::Delete,
                                        buffer::now_secs(),
                                    );
                                }
                                project_search_query.pop();
                                project_search_results = project_search::run_project_search(
                                    &project_search_query,
                                    &project_root,
                                    project_use_regex,
                                    project_whole_word,
                                    project_case_insensitive,
                                );
                                project_search_selected = 0;
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Project replace intercepts keys when active.
                    if subsystems.has_find_in_files() && project_replace_active {
                        if mods.alt && !mods.ctrl {
                            let toggled = match key.as_str() {
                                "r" => {
                                    project_use_regex = !project_use_regex;
                                    true
                                }
                                "w" => {
                                    project_whole_word = !project_whole_word;
                                    true
                                }
                                "i" => {
                                    project_case_insensitive = !project_case_insensitive;
                                    true
                                }
                                _ => false,
                            };
                            if toggled {
                                project_replace_results = project_search::run_project_search(
                                    &project_replace_search,
                                    &project_root,
                                    project_use_regex,
                                    project_whole_word,
                                    project_case_insensitive,
                                );
                                project_replace_selected = 0;
                                redraw = true;
                                continue;
                            }
                        }
                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            if project_replace_focus_on_replace {
                                let restored = if is_redo {
                                    project_replace_with_history
                                        .redo(&project_replace_with, project_replace_with.len())
                                } else {
                                    project_replace_with_history
                                        .undo(&project_replace_with, project_replace_with.len())
                                };
                                if let Some((t, _)) = restored {
                                    project_replace_with = t;
                                }
                            } else {
                                let restored = if is_redo {
                                    project_replace_search_history
                                        .redo(&project_replace_search, project_replace_search.len())
                                } else {
                                    project_replace_search_history
                                        .undo(&project_replace_search, project_replace_search.len())
                                };
                                if let Some((t, _)) = restored {
                                    project_replace_search = t;
                                    project_replace_results = project_search::run_project_search(
                                        &project_replace_search,
                                        &project_root,
                                        project_use_regex,
                                        project_whole_word,
                                        project_case_insensitive,
                                    );
                                    project_replace_selected = 0;
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            match action {
                                FieldClipboard::Copy => {
                                    let src = if project_replace_focus_on_replace {
                                        &project_replace_with
                                    } else {
                                        &project_replace_search
                                    };
                                    if !src.is_empty() {
                                        crate::window::set_clipboard_text(src);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if project_replace_focus_on_replace {
                                        if !project_replace_with.is_empty() {
                                            crate::window::set_clipboard_text(
                                                &project_replace_with,
                                            );
                                            project_replace_with_history.record(
                                                &project_replace_with,
                                                project_replace_with.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            project_replace_with.clear();
                                        }
                                    } else if !project_replace_search.is_empty() {
                                        crate::window::set_clipboard_text(&project_replace_search);
                                        project_replace_search_history.record(
                                            &project_replace_search,
                                            project_replace_search.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        project_replace_search.clear();
                                        project_replace_results = project_search::run_project_search(
                                            &project_replace_search,
                                            &project_root,
                                            project_use_regex,
                                            project_whole_word,
                                            project_case_insensitive,
                                        );
                                        project_replace_selected = 0;
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        if project_replace_focus_on_replace {
                                            project_replace_with_history.record(
                                                &project_replace_with,
                                                project_replace_with.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            append_clipboard_line(&mut project_replace_with, &clip);
                                        } else {
                                            project_replace_search_history.record(
                                                &project_replace_search,
                                                project_replace_search.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            append_clipboard_line(
                                                &mut project_replace_search,
                                                &clip,
                                            );
                                            project_replace_results = project_search::run_project_search(
                                                &project_replace_search,
                                                &project_root,
                                                project_use_regex,
                                                project_whole_word,
                                                project_case_insensitive,
                                            );
                                            project_replace_selected = 0;
                                        }
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        match key.as_str() {
                            "escape" => {
                                project_replace_active = false;
                            }
                            "tab" => {
                                project_replace_focus_on_replace =
                                    !project_replace_focus_on_replace;
                            }
                            "return" | "keypad enter" if mods.ctrl
                                // Execute replace all.
                                && !project_replace_search.is_empty() => {
                                    // Run the project-wide sed on a worker
                                    // thread; its count and the doc reload are
                                    // applied from the per-frame poll.
                                    if replace_job.is_none() {
                                        let root = project_root.clone();
                                        let search = project_replace_search.clone();
                                        let with = project_replace_with.clone();
                                        let use_regex = project_use_regex;
                                        let case_insensitive = project_case_insensitive;
                                        replace_job = Some(std::thread::spawn(move || {
                                            project_search::execute_project_replace(
                                                &root,
                                                &search,
                                                &with,
                                                use_regex,
                                                case_insensitive,
                                            )
                                        }));
                                        info_message = Some((
                                            "Replacing across project...".to_string(),
                                            Instant::now(),
                                        ));
                                    }
                                    project_replace_active = false;
                                }
                            "return" | "keypad enter"
                                // Preview: run search to show matches.
                                if !project_replace_search.is_empty() => {
                                    project_replace_results = project_search::run_project_search(
                                        &project_replace_search,
                                        &project_root,
                                        project_use_regex,
                                        project_whole_word,
                                        project_case_insensitive,
                                    );
                                    project_replace_selected = 0;
                                }
                            "up" => {
                                project_replace_selected =
                                    project_replace_selected.saturating_sub(1);
                            }
                            "down"
                                if !project_replace_results.is_empty() => {
                                    project_replace_selected = (project_replace_selected + 1)
                                        .min(project_replace_results.len() - 1);
                                }
                            "backspace" => {
                                if project_replace_focus_on_replace {
                                    if !project_replace_with.is_empty() {
                                        project_replace_with_history.record(
                                            &project_replace_with,
                                            project_replace_with.len(),
                                            FieldEdit::Delete,
                                            buffer::now_secs(),
                                        );
                                    }
                                    project_replace_with.pop();
                                } else {
                                    if !project_replace_search.is_empty() {
                                        project_replace_search_history.record(
                                            &project_replace_search,
                                            project_replace_search.len(),
                                            FieldEdit::Delete,
                                            buffer::now_secs(),
                                        );
                                    }
                                    project_replace_search.pop();
                                    project_replace_results = project_search::run_project_search(
                                        &project_replace_search,
                                        &project_root,
                                        project_use_regex,
                                        project_whole_word,
                                        project_case_insensitive,
                                    );
                                    project_replace_selected = 0;
                                }
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Code-action picker intercepts keys.
                    if code_action_active {
                        match key.as_str() {
                            "escape" => {
                                code_action_active = false;
                            }
                            "up" => {
                                code_action_selected = code_action_selected.saturating_sub(1);
                            }
                            "down" if !code_actions.is_empty() => {
                                code_action_selected =
                                    (code_action_selected + 1).min(code_actions.len() - 1);
                            }
                            "return" | "keypad enter" => {
                                if let Some((_, action)) =
                                    code_actions.get(code_action_selected).cloned()
                                {
                                    let atomic = config.files.atomic_save;
                                    if let Some(edit) = action.get("edit") {
                                        let n = apply_lsp_workspace_edit(
                                            edit,
                                            &mut docs,
                                            use_git(),
                                            atomic,
                                        );
                                        if n > 0 {
                                            for d in &mut docs {
                                                d.cached_change_id = -1;
                                            }
                                            crate::window::force_invalidate();
                                        }
                                    }
                                    if let Some(tid) = lsp_state.transport_id {
                                        let cmdv = action.get("command");
                                        let (name, args) = if let Some(s) =
                                            cmdv.and_then(|c| c.as_str())
                                        {
                                            (Some(s.to_string()), action.get("arguments").cloned())
                                        } else if let Some(obj) = cmdv.filter(|c| c.is_object()) {
                                            (
                                                obj.get("command")
                                                    .and_then(|v| v.as_str())
                                                    .map(String::from),
                                                obj.get("arguments").cloned(),
                                            )
                                        } else {
                                            (None, None)
                                        };
                                        if let Some(name) = name {
                                            let req_id = lsp_state.next_id();
                                            lsp_state.pending_requests.insert(
                                                req_id,
                                                "workspace/executeCommand".to_string(),
                                            );
                                            let _ = lsp::send_message(
                                                tid,
                                                &serde_json::json!({
                                                    "jsonrpc": "2.0",
                                                    "id": req_id,
                                                    "method": "workspace/executeCommand",
                                                    "params": {
                                                        "command": name,
                                                        "arguments":
                                                            args.unwrap_or_else(|| serde_json::json!([]))
                                                    }
                                                }),
                                            );
                                        }
                                    }
                                }
                                code_action_active = false;
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Git status view intercepts keys.
                    if subsystems.has_git() && git_status_active {
                        match key.as_str() {
                            "escape" => {
                                git_status_active = false;
                            }
                            "return" | "keypad enter" => {
                                if let Some((_, path, _)) =
                                    git_status_entries.get(git_status_selected).cloned()
                                {
                                    git_status_active = false;
                                    let full_path = format!("{project_root}/{path}");
                                    let tab_idx = docs.iter().position(|d| d.path == full_path);
                                    let idx = if let Some(i) = tab_idx {
                                        i
                                    } else if open_file_into(&full_path, &mut docs, use_git()) {
                                        autoreload.watch(&full_path);
                                        remember_recent_file(
                                            &mut recent_files,
                                            &full_path,
                                            userdir_path,
                                        );
                                        docs.len() - 1
                                    } else {
                                        redraw = true;
                                        continue;
                                    };
                                    active_tab = idx;
                                }
                            }
                            "up" => {
                                git_status_selected = git_status_selected.saturating_sub(1);
                            }
                            "down" if !git_status_entries.is_empty() => {
                                git_status_selected =
                                    (git_status_selected + 1).min(git_status_entries.len() - 1);
                            }
                            "r" | "R" => {
                                if git_status_job.is_none() {
                                    let root = project_root.clone();
                                    git_status_job =
                                        Some(std::thread::spawn(move || git_helpers::run_git_status(&root)));
                                }
                                git_status_selected = 0;
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Git log view intercepts keys when active.
                    if subsystems.has_git() && git_log_active {
                        match key.as_str() {
                            "escape" => {
                                git_log_active = false;
                            }
                            "up" => {
                                git_log_selected = git_log_selected.saturating_sub(1);
                            }
                            "down" if !git_log_entries.is_empty() => {
                                git_log_selected =
                                    (git_log_selected + 1).min(git_log_entries.len() - 1);
                            }
                            _ => {}
                        }
                        redraw = true;
                        continue;
                    }

                    // Terminal intercepts all keys when focused.
                    if terminal.visible && terminal.focused {
                        // Ctrl+PageUp/PageDown switch terminal tabs.
                        if mods.ctrl && !mods.alt && !mods.shift {
                            match key.as_str() {
                                "pageup" => {
                                    terminal.prev_tab();
                                    redraw = true;
                                    continue;
                                }
                                "pagedown" => {
                                    terminal.next_tab();
                                    redraw = true;
                                    continue;
                                }
                                _ => {}
                            }
                        }
                        // Terminal Ctrl+Shift+A: select every visible cell
                        // so the user can copy the current viewport
                        // (including whatever scrollback is currently
                        // shown) without dragging across it manually. The
                        // gnome-terminal / xterm convention. Plain Ctrl+A
                        // stays as the shell's "move to line start" so
                        // the shell is still usable.
                        if mods.ctrl && mods.shift && !mods.alt && key == "a" {
                            let (_, wh, _, _) = crate::window::get_window_size();
                            let win_h = wh as f64;
                            let status_h_a = style.font_height + style.padding_y * 2.0;
                            let tab_h_a = if !docs.is_empty() {
                                style.font_height + style.padding_y * 3.0
                            } else {
                                0.0
                            };
                            let terminal_h_a = terminal_h_override
                                .unwrap_or(
                                    (win_h * 0.3)
                                        .min(win_h - tab_h_a - status_h_a - 50.0)
                                        .max(80.0),
                                )
                                .min(win_h - tab_h_a - status_h_a - 50.0)
                                .max(80.0);
                            let tab_bar_h_a = if !terminal.terminals.is_empty() {
                                style.font_height + style.padding_y * 3.0
                            } else {
                                0.0
                            };
                            let char_h_a = style.line_height();
                            let rows_visible = (((terminal_h_a
                                - style.divider_size
                                - tab_bar_h_a
                                - style.padding_y * 2.0)
                                / char_h_a)
                                .floor()
                                .max(1.0)) as usize;
                            if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                                inst.sel_start = Some((0, 0));
                                inst.sel_end = Some((rows_visible.saturating_sub(1), usize::MAX));
                                inst.sel_dragging = false;
                            }
                            redraw = true;
                            continue;
                        }
                        // Terminal copy / paste.
                        //   Ctrl+Shift+C  or  Ctrl+Insert : copy selection
                        //   Ctrl+Shift+V  or  Shift+Insert: paste clipboard
                        // Plain Ctrl+C / Ctrl+V remain sent to the shell
                        // (SIGINT / literal, respectively).
                        let is_copy_combo = mods.ctrl
                            && ((mods.shift && key == "c") || (!mods.shift && key == "insert"));
                        let is_paste_combo = mods.shift
                            && ((mods.ctrl && key == "v") || (!mods.ctrl && key == "insert"));
                        if is_copy_combo {
                            if let Some(inst) = terminal.terminals.get(terminal.active) {
                                if let (Some(s), Some(e)) = (inst.sel_start, inst.sel_end) {
                                    if let Some((a, b)) =
                                        crate::editor::terminal_panel::normalized_selection(s, e)
                                    {
                                        let cap = inst.tbuf.history_len() as f64;
                                        let scrollback_rows =
                                            inst.scrollback.round().max(0.0).min(cap) as usize;
                                        // Recompute rows_visible from current geometry.
                                        let (_, wh, _, _) = crate::window::get_window_size();
                                        let win_h = wh as f64;
                                        let status_h_c = style.font_height + style.padding_y * 2.0;
                                        let tab_h_c = if !docs.is_empty() {
                                            style.font_height + style.padding_y * 3.0
                                        } else {
                                            0.0
                                        };
                                        let terminal_h_c = terminal_h_override
                                            .unwrap_or(
                                                (win_h * 0.3)
                                                    .min(win_h - tab_h_c - status_h_c - 50.0)
                                                    .max(80.0),
                                            )
                                            .min(win_h - tab_h_c - status_h_c - 50.0)
                                            .max(80.0);
                                        let tab_bar_h_c = if !terminal.terminals.is_empty() {
                                            style.font_height + style.padding_y * 3.0
                                        } else {
                                            0.0
                                        };
                                        let char_h_c = style.line_height();
                                        let rows_visible = (((terminal_h_c
                                            - style.divider_size
                                            - tab_bar_h_c
                                            - style.padding_y * 2.0)
                                            / char_h_c)
                                            .floor()
                                            .max(1.0))
                                            as usize;
                                        let rows_data =
                                            inst.tbuf.visible_rows(rows_visible, scrollback_rows);
                                        let text =
                                            crate::editor::terminal_panel::extract_selection_text(
                                                &rows_data, a, b,
                                            );
                                        if !text.is_empty() {
                                            crate::window::set_clipboard_text(&text);
                                        }
                                    }
                                }
                            }
                            if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                                inst.sel_start = None;
                                inst.sel_end = None;
                                inst.sel_dragging = false;
                            }
                            redraw = true;
                            continue;
                        }
                        if is_paste_combo {
                            if let Some(text) = crate::window::get_clipboard_text() {
                                if let Some(inst) = terminal.active_terminal() {
                                    let _ = inst.inner.write(text.as_bytes());
                                    inst.scrollback = 0.0;
                                    inst.scrollback_target = 0.0;
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(inst) = terminal.active_terminal() {
                            let data = match key.as_str() {
                                "return" | "keypad enter" => Some(b"\r".to_vec()),
                                "backspace" => Some(vec![0x7f]),
                                "tab" => Some(b"\t".to_vec()),
                                "up" => Some(b"\x1b[A".to_vec()),
                                "down" => Some(b"\x1b[B".to_vec()),
                                "right" => Some(b"\x1b[C".to_vec()),
                                "left" => Some(b"\x1b[D".to_vec()),
                                "delete" => Some(b"\x1b[3~".to_vec()),
                                "home" => Some(b"\x1b[H".to_vec()),
                                "end" => Some(b"\x1b[F".to_vec()),
                                _ => {
                                    if key.len() == 1 {
                                        let ch = key.as_bytes()[0];
                                        if mods.ctrl {
                                            // Ctrl+letter -> control char.
                                            let ctrl = ch & 0x1f;
                                            Some(vec![ctrl])
                                        } else {
                                            None // Handled by TextInput
                                        }
                                    } else {
                                        None
                                    }
                                }
                            };
                            if let Some(bytes) = data {
                                let _ = inst.inner.write(&bytes);
                                // Snap to live bottom so the caret is visible.
                                inst.scrollback = 0.0;
                                inst.scrollback_target = 0.0;
                            }
                        }
                        redraw = true;
                        continue;
                    }

                    // Dismiss info message on any key.
                    if info_message.is_some() {
                        info_message = None;
                        redraw = true;
                        if key == "escape" {
                            continue;
                        }
                    }

                    // "No extension detected, save anyway?" prompt. Yes runs
                    // the overwrite check next (and the save if that
                    // passes); No just dismisses the nag so the user can
                    // type `.ext` in the picker and press Enter again.
                    if let Nag::NoExtension { save_path, doc_tab } = &nag {
                        let save_path = save_path.clone();
                        let tab = *doc_tab;
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" | "return" | "keypad enter" => {
                                // Chain into the overwrite path: if the file
                                // exists and isn't the current doc's own
                                // path, hand off to OverwriteFile; otherwise
                                // perform the save directly.
                                let own_path = docs
                                    .get(tab)
                                    .map(|d| d.path.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if std::path::Path::new(&save_path).is_file()
                                    && save_path != own_path
                                {
                                    nag = Nag::OverwriteFile {
                                        save_path,
                                        doc_tab: tab,
                                    };
                                    redraw = true;
                                    continue;
                                }
                                if let Some(doc) = docs.get_mut(tab) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let atomic = config.files.atomic_save;
                                        let saved_id = buffer::with_buffer(buf_id, |b| {
                                            buffer::save_file(b, &save_path, b.crlf, atomic)
                                                .map_err(|_| buffer::BufferError::UnknownBuffer)?;
                                            Ok(b.change_id)
                                        });
                                        if let Ok(id) = saved_id {
                                            doc.saved_change_id = id;
                                            doc.saved_signature =
                                                buffer::with_buffer(buf_id, |b| {
                                                    Ok(buffer::content_signature(&b.lines))
                                                })
                                                .unwrap_or(0);
                                            doc.path = save_path.clone();
                                            doc.name = std::path::Path::new(&save_path)
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_else(|| save_path.clone());
                                            doc.cached_change_id = -1;
                                            doc.cached_render = std::sync::Arc::new(Vec::new());
                                            autoreload.watch(&save_path);
                                            log_to_file(userdir, &format!("Saved {save_path}"));
                                            info_message = Some((
                                                format!("Saved {}", doc.name),
                                                Instant::now(),
                                            ));
                                        } else {
                                            info_message = Some((
                                                format!("Failed to save {save_path}"),
                                                Instant::now(),
                                            ));
                                        }
                                    }
                                }
                                nag = Nag::None;
                                cmdview_active = false;
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // "Delete FILE?" prompt intercepts keys. Yes removes the
                    // file from disk and any open tab; No dismisses.
                    if let Nag::DeleteFile { path } = &nag {
                        let target = path.clone();
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" | "return" | "keypad enter" => {
                                match std::fs::remove_file(&target) {
                                    Ok(()) => {
                                        autoreload.unwatch(&target);
                                        let mut i = 0;
                                        while i < docs.len() {
                                            if docs[i].path == target {
                                                docs.remove(i);
                                                if active_tab >= docs.len() && !docs.is_empty() {
                                                    active_tab = docs.len() - 1;
                                                } else if docs.is_empty() {
                                                    active_tab = 0;
                                                } else if i < active_tab {
                                                    active_tab = active_tab.saturating_sub(1);
                                                }
                                            } else {
                                                i += 1;
                                            }
                                        }
                                        if subsystems.has_sidebar() && !project_root.is_empty() {
                                            let in_memory_expanded: HashSet<String> =
                                                sidebar_entries
                                                    .iter()
                                                    .filter(|e| e.is_dir && e.expanded)
                                                    .map(|e| e.path.clone())
                                                    .collect();
                                            sidebar_entries = scan_for_sidebar(
                                                                                                &project_root,
                                                sidebar_show_hidden,
                                            );
                                            restore_expanded_folders(
                                                &mut sidebar_entries,
                                                userdir_path,
                                                sidebar_show_hidden,
                                                &project_session_key(&project_root),
                                            );
                                            expand_sidebar_from_set(
                                                &mut sidebar_entries,
                                                &in_memory_expanded,
                                                sidebar_show_hidden,
                                            );
                                        }
                                        info_message =
                                            Some((format!("Deleted {target}"), Instant::now()));
                                    }
                                    Err(e) => {
                                        info_message =
                                            Some((format!("Delete failed: {e}"), Instant::now()));
                                    }
                                }
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // "Overwrite FILE?" prompt intercepts keys. Yes writes
                    // over the existing file; No returns to the Save As
                    // picker so the user can adjust the filename. Escape /N
                    // just dismisses the nag (keeps cmdview open).
                    if let Nag::OverwriteFile { save_path, doc_tab } = &nag {
                        let save_path = save_path.clone();
                        let tab = *doc_tab;
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" | "return" | "keypad enter" => {
                                if let Some(doc) = docs.get_mut(tab) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let atomic = config.files.atomic_save;
                                        let saved_id = buffer::with_buffer(buf_id, |b| {
                                            buffer::save_file(b, &save_path, b.crlf, atomic)
                                                .map_err(|_| buffer::BufferError::UnknownBuffer)?;
                                            Ok(b.change_id)
                                        });
                                        if let Ok(id) = saved_id {
                                            doc.saved_change_id = id;
                                            doc.saved_signature =
                                                buffer::with_buffer(buf_id, |b| {
                                                    Ok(buffer::content_signature(&b.lines))
                                                })
                                                .unwrap_or(0);
                                            doc.path = save_path.clone();
                                            doc.name = std::path::Path::new(&save_path)
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_else(|| save_path.clone());
                                            doc.cached_change_id = -1;
                                            doc.cached_render = std::sync::Arc::new(Vec::new());
                                            autoreload.watch(&save_path);
                                            log_to_file(userdir, &format!("Saved {save_path}"));
                                            info_message = Some((
                                                format!("Saved {}", doc.name),
                                                Instant::now(),
                                            ));
                                        } else {
                                            info_message = Some((
                                                format!("Failed to save {save_path}"),
                                                Instant::now(),
                                            ));
                                        }
                                    }
                                }
                                nag = Nag::None;
                                cmdview_active = false;
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                // Back off to the picker — cmdview stays
                                // open with the text the user typed so they
                                // can rename.
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // "Create missing directory?" prompt intercepts keys when
                    // active. Yes creates the parent tree and performs the
                    // pending save; No backs off without writing. Escape /N
                    // also closes the originating Save As picker so the user
                    // is not left staring at it.
                    if let Nag::CreateDir {
                        parent: parent_str,
                        save_path,
                        doc_tab,
                        from_save_as,
                    } = &nag
                    {
                        let save_path = save_path.clone();
                        let parent_str = parent_str.clone();
                        let tab = *doc_tab;
                        let is_save_as = *from_save_as;
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" | "return" | "keypad enter" => {
                                let parent = std::path::Path::new(&save_path)
                                    .parent()
                                    .map(|p| p.to_path_buf());
                                let create_ok = match parent {
                                    Some(p) => std::fs::create_dir_all(&p).is_ok(),
                                    None => true,
                                };
                                if !create_ok {
                                    info_message = Some((
                                        format!("Could not create directory {parent_str}"),
                                        Instant::now(),
                                    ));
                                    nag = Nag::None;
                                    if is_save_as {
                                        cmdview_active = false;
                                    }
                                    redraw = true;
                                    continue;
                                }
                                if let Some(doc) = docs.get_mut(tab) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let atomic = config.files.atomic_save;
                                        let saved_id = buffer::with_buffer(buf_id, |b| {
                                            buffer::save_file(b, &save_path, b.crlf, atomic)
                                                .map_err(|_| buffer::BufferError::UnknownBuffer)?;
                                            Ok(b.change_id)
                                        });
                                        if let Ok(id) = saved_id {
                                            doc.saved_change_id = id;
                                            doc.saved_signature =
                                                buffer::with_buffer(buf_id, |b| {
                                                    Ok(buffer::content_signature(&b.lines))
                                                })
                                                .unwrap_or(0);
                                            if is_save_as {
                                                doc.path = save_path.clone();
                                                doc.name = std::path::Path::new(&save_path)
                                                    .file_name()
                                                    .map(|n| n.to_string_lossy().to_string())
                                                    .unwrap_or_else(|| save_path.clone());
                                                doc.cached_change_id = -1;
                                                doc.cached_render = std::sync::Arc::new(Vec::new());
                                            }
                                            autoreload.watch(&save_path);
                                            log_to_file(userdir, &format!("Saved {save_path}"));
                                            info_message = Some((
                                                format!("Saved {}", doc.name),
                                                Instant::now(),
                                            ));
                                            if !is_save_as && subsystems.has_git() {
                                                // Diff off the UI thread; the
                                                // gutter fills in via drain_diffs.
                                                crate::editor::git::start_diff(&save_path);
                                            }
                                        } else {
                                            info_message = Some((
                                                format!("Failed to save {save_path}"),
                                                Instant::now(),
                                            ));
                                        }
                                    }
                                }
                                nag = Nag::None;
                                if is_save_as {
                                    cmdview_active = false;
                                }
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                nag = Nag::None;
                                if is_save_as {
                                    cmdview_active = false;
                                }
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Nag view intercepts keys when active; dismiss any overlay.
                    if let Nag::UnsavedChanges { tab_to_close, .. } = &nag {
                        let tab_to_close = *tab_to_close;
                        cmdview_active = false;
                        palette_active = false;
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" | "return" | "keypad enter" => {
                                // Yes: discard unsaved changes and proceed.
                                if let Some(idx) = tab_to_close {
                                    if let Some(d) = docs.get(idx) {
                                        autoreload.unwatch(&d.path);
                                    }
                                    docs.remove(idx);
                                    if docs.is_empty() {
                                        active_tab = 0;
                                    } else if idx <= active_tab {
                                        active_tab = active_tab.saturating_sub(1);
                                    }
                                } else {
                                    quit = true;
                                }
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                // No / Cancel: leave everything as-is.
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Reload nag intercepts keys when active.
                    if let Nag::ReloadFromDisk { path } = &nag {
                        let rpath = path.clone();
                        // Every arm here resolves the keystroke, so swallow
                        // the follow-on TextInput regardless of which arm
                        // matches.
                        eat_next_text_input = true;
                        match key.as_str() {
                            "y" | "Y" => {
                                // Reload from disk.
                                if let Some(doc) = docs.iter_mut().find(|d| d.path == rpath) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let _ = buffer::with_buffer_mut(buf_id, |b| {
                                            let mut buf_state = buffer::default_buffer_state();
                                            if buffer::load_file(&mut buf_state, &rpath).is_ok() {
                                                b.lines = buf_state.lines;
                                                // See autoreload path: bump change_id past
                                                // its current value so the render cache
                                                // doesn't hit on the stale lines.
                                                b.change_id = b.change_id.wrapping_add(1).max(1);
                                            }
                                            Ok(())
                                        });
                                        doc.cached_change_id = -1;
                                        doc.cached_render = std::sync::Arc::new(Vec::new());
                                        if let Ok((cid, sig)) = buffer::with_buffer(buf_id, |b| {
                                            Ok((b.change_id, buffer::content_signature(&b.lines)))
                                        }) {
                                            doc.saved_change_id = cid;
                                            doc.saved_signature = sig;
                                        }
                                    }
                                }
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            "n" | "N" | "escape" => {
                                nag = Nag::None;
                                redraw = true;
                                continue;
                            }
                            _ => {
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Command palette intercepts keys when active.
                    if palette_active {
                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            let restored = if is_redo {
                                palette_history.redo(&palette_query, palette_query.len())
                            } else {
                                palette_history.undo(&palette_query, palette_query.len())
                            };
                            if let Some((t, _)) = restored {
                                palette_query = t;
                                palette_results =
                                    fuzzy_filter_commands(&palette_query, &all_commands);
                                palette_selected =
                                    palette_selected.min(palette_results.len().saturating_sub(1));
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            match action {
                                FieldClipboard::Copy => {
                                    if !palette_query.is_empty() {
                                        crate::window::set_clipboard_text(&palette_query);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if !palette_query.is_empty() {
                                        crate::window::set_clipboard_text(&palette_query);
                                        palette_history.record(
                                            &palette_query,
                                            palette_query.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        palette_query.clear();
                                        palette_results =
                                            fuzzy_filter_commands(&palette_query, &all_commands);
                                        palette_selected = palette_selected
                                            .min(palette_results.len().saturating_sub(1));
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        palette_history.record(
                                            &palette_query,
                                            palette_query.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        append_clipboard_line(&mut palette_query, &clip);
                                        palette_results =
                                            fuzzy_filter_commands(&palette_query, &all_commands);
                                        palette_selected = palette_selected
                                            .min(palette_results.len().saturating_sub(1));
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        // Check if the key is handled by the palette.
                        let palette_handled = match key.as_str() {
                            "escape" => {
                                palette_active = false;
                                true
                            }
                            "return" | "keypad enter" => {
                                if let Some((cmd, _)) = palette_results.get(palette_selected) {
                                    let cmd = cmd.clone();
                                    palette_active = false;
                                    // If the selected item is a file path, open it.
                                    if cmd.starts_with('/') && std::path::Path::new(&cmd).is_file()
                                    {
                                        if open_file_into(&cmd, &mut docs, use_git()) {
                                            active_tab = docs.len() - 1;
                                            autoreload.watch(&cmd);
                                            remember_recent_file(
                                                &mut recent_files,
                                                &cmd,
                                                userdir_path,
                                            );
                                        }
                                    } else {
                                        // Execute the selected command.
                                        let cmd: String = cmd;
                                        include!("commands_dispatch.rs");
                                    }
                                }
                                true
                            }
                            "backspace" => {
                                if !palette_query.is_empty() {
                                    palette_history.record(
                                        &palette_query,
                                        palette_query.len(),
                                        FieldEdit::Delete,
                                        buffer::now_secs(),
                                    );
                                }
                                palette_query.pop();
                                false
                            }
                            "up" => {
                                palette_selected = palette_selected.saturating_sub(1);
                                true
                            }
                            "down" => {
                                if palette_selected + 1 < palette_results.len() {
                                    palette_selected += 1;
                                }
                                true
                            }
                            _ => false,
                        };
                        if palette_handled {
                            // Filter commands with fuzzy matching.
                            palette_results = fuzzy_filter_commands(&palette_query, &all_commands);
                            palette_selected =
                                palette_selected.min(palette_results.len().saturating_sub(1));
                            redraw = true;
                            continue;
                        }
                        // Unhandled key: let keymap dispatch handle it.
                        // If a keymap command fires, it will close the palette.
                    }

                    // Theme picker intercepts keys when active.
                    if theme_picker_active {
                        let theme_picker_handled = match key.as_str() {
                            "escape" => {
                                // Restore original theme.
                                if let Some(orig) = theme_picker_original_style.take() {
                                    style = orig;
                                    current_theme_idx = theme_picker_original_idx;
                                }
                                theme_picker_active = false;
                                true
                            }
                            "return" | "keypad enter" => {
                                // Confirm selected theme.
                                if let Some((name, _)) = theme_picker_results.get(theme_picker_selected) {
                                    current_theme_idx = available_themes.iter().position(|t| t == name).unwrap_or(0);
                                    // Persist to config.toml
                                    let config_path = std::path::Path::new(userdir).join("config.toml");
                                    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
                                    if let Ok(mut doc) = existing.parse::<toml::Value>() {
                                        if let toml::Value::Table(ref mut map) = doc {
                                            map.insert("theme".to_string(), toml::Value::String(name.clone()));
                                        }
                                        if let Ok(out) = toml::to_string(&doc) {
                                            let _ = std::fs::write(&config_path, out);
                                        }
                                    }
                                }
                                theme_picker_active = false;
                                theme_picker_original_style = None;
                                true
                            }
                            "backspace" => {
                                theme_picker_query.pop();
                                true
                            }
                            "up" => {
                                theme_picker_selected = theme_picker_selected.saturating_sub(1);
                                true
                            }
                            "down" => {
                                if theme_picker_selected + 1 < theme_picker_results.len() {
                                    theme_picker_selected += 1;
                                }
                                true
                            }
                            _ => false,
                        };
                        if theme_picker_handled {
                            // Refilter results if query changed.
                            theme_picker_results = if theme_picker_query.is_empty() {
                                available_themes.iter().map(|t| (t.clone(), t.clone())).collect()
                            } else {
                                let q = theme_picker_query.to_lowercase();
                                available_themes.iter().filter(|t| {
                                    t.to_lowercase().contains(&q)
                                }).map(|t| (t.clone(), t.clone())).collect()
                            };
                            theme_picker_selected = theme_picker_selected.min(theme_picker_results.len().saturating_sub(1));
                            // Preview the selected theme.
                            if let Some((name, _)) = theme_picker_results.get(theme_picker_selected) {
                                let tp = Path::new(datadir)
                                    .join("assets")
                                    .join("themes")
                                    .join(format!("{name}.json"))
                                    .to_string_lossy()
                                    .into_owned();
                                if let Ok(palette) = crate::editor::style::load_theme_palette(&tp) {
                                    apply_theme_to_style(&mut style, &palette);
                                    crate::editor::style_ctx::set_current_style(style.clone());
                                    // Invalidate all render caches so syntax colours refresh.
                                    pending_render_cache = None;
                                    for doc in &mut docs {
                                        doc.cached_change_id = -1;
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        // Unhandled key: let keymap dispatch handle it.
                    }

                    // Find bar intercepts keys when active.
                    if find_active {
                        // Alt-chorded toggles apply regardless of which input has focus.
                        if mods.alt && !mods.ctrl {
                            let toggled = match key.as_str() {
                                "r" => {
                                    find_use_regex = !find_use_regex;
                                    true
                                }
                                "w" => {
                                    find_whole_word = !find_whole_word;
                                    true
                                }
                                "i" => {
                                    find_case_insensitive = !find_case_insensitive;
                                    true
                                }
                                "s" => {
                                    find_in_selection = !find_in_selection;
                                    if find_in_selection && find_selection_range.is_none() {
                                        // Capture current selection if we don't already have one.
                                        if let Some(doc) = docs.get(active_tab) {
                                            let a = doc_anchor(&doc.view);
                                            let c = doc_cursor(&doc.view);
                                            if a.0 != c.0 {
                                                let (sl, sc) = if a < c { a } else { c };
                                                let (el, ec) = if a < c { c } else { a };
                                                find_selection_range = Some((sl, sc, el, ec));
                                            } else {
                                                // Single-line selection; not meaningful for
                                                // find-in-selection. Disable again.
                                                find_in_selection = false;
                                            }
                                        }
                                    }
                                    true
                                }
                                _ => false,
                            };
                            if toggled {
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    let sel = if find_in_selection {
                                        find_selection_range
                                    } else {
                                        None
                                    };
                                    find_matches = compute_find_matches_filtered(
                                        dv,
                                        &find_query,
                                        find_use_regex,
                                        find_whole_word,
                                        find_case_insensitive,
                                        sel,
                                    );
                                    find_current = find_match_at_or_after(
                                        &find_matches,
                                        find_anchor.0,
                                        find_anchor.1,
                                    );
                                    if let Some(i) = find_current {
                                        select_find_match(dv, find_matches[i], replace_active);
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                        }
                        if let Some(is_redo) = keymap_field_undo(&keymap, key.as_str(), mods) {
                            // Route the undo/redo bindings to the focused field
                            // instead of letting them leak through to the document.
                            if find_focus_on_replace {
                                let restored = if is_redo {
                                    replace_history.redo(&replace_query, replace_query.len())
                                } else {
                                    replace_history.undo(&replace_query, replace_query.len())
                                };
                                if let Some((t, _)) = restored {
                                    replace_query = t;
                                }
                            } else {
                                let restored = if is_redo {
                                    find_history.redo(&find_query, find_query.len())
                                } else {
                                    find_history.undo(&find_query, find_query.len())
                                };
                                if let Some((t, _)) = restored {
                                    find_query = t;
                                    if let Some(doc) = docs.get_mut(active_tab) {
                                        let dv = &mut doc.view;
                                        let sel = if find_in_selection {
                                            find_selection_range
                                        } else {
                                            None
                                        };
                                        find_matches = compute_find_matches_filtered(
                                            dv,
                                            &find_query,
                                            find_use_regex,
                                            find_whole_word,
                                            find_case_insensitive,
                                            sel,
                                        );
                                        find_current = find_match_at_or_after(
                                            &find_matches,
                                            find_anchor.0,
                                            find_anchor.1,
                                        );
                                        if let Some(i) = find_current {
                                            select_find_match(dv, find_matches[i], replace_active);
                                        }
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        if let Some(action) = keymap_field_clipboard(&keymap, key.as_str(), mods) {
                            // Clipboard ops belong to the find bar while it holds
                            // focus, not the document.
                            match action {
                                FieldClipboard::Copy => {
                                    let src = if find_focus_on_replace {
                                        &replace_query
                                    } else {
                                        &find_query
                                    };
                                    if !src.is_empty() {
                                        crate::window::set_clipboard_text(src);
                                    }
                                }
                                FieldClipboard::Cut => {
                                    if find_focus_on_replace {
                                        if !replace_query.is_empty() {
                                            crate::window::set_clipboard_text(&replace_query);
                                            replace_history.record(
                                                &replace_query,
                                                replace_query.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            replace_query.clear();
                                        }
                                    } else if !find_query.is_empty() {
                                        crate::window::set_clipboard_text(&find_query);
                                        find_history.record(
                                            &find_query,
                                            find_query.len(),
                                            FieldEdit::Replace,
                                            buffer::now_secs(),
                                        );
                                        find_query.clear();
                                        if let Some(doc) = docs.get_mut(active_tab) {
                                            let dv = &mut doc.view;
                                            let sel = if find_in_selection {
                                                find_selection_range
                                            } else {
                                                None
                                            };
                                            find_matches = compute_find_matches_filtered(
                                                dv,
                                                &find_query,
                                                find_use_regex,
                                                find_whole_word,
                                                find_case_insensitive,
                                                sel,
                                            );
                                            find_current = find_match_at_or_after(
                                                &find_matches,
                                                find_anchor.0,
                                                find_anchor.1,
                                            );
                                            if let Some(i) = find_current {
                                                select_find_match(
                                                    dv,
                                                    find_matches[i],
                                                    replace_active,
                                                );
                                            }
                                        }
                                    }
                                }
                                FieldClipboard::Paste => {
                                    if let Some(clip) = crate::window::get_clipboard_text() {
                                        if find_focus_on_replace {
                                            replace_history.record(
                                                &replace_query,
                                                replace_query.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            append_clipboard_line(&mut replace_query, &clip);
                                        } else {
                                            find_history.record(
                                                &find_query,
                                                find_query.len(),
                                                FieldEdit::Replace,
                                                buffer::now_secs(),
                                            );
                                            append_clipboard_line(&mut find_query, &clip);
                                            if let Some(doc) = docs.get_mut(active_tab) {
                                                let dv = &mut doc.view;
                                                let sel = if find_in_selection {
                                                    find_selection_range
                                                } else {
                                                    None
                                                };
                                                find_matches = compute_find_matches_filtered(
                                                    dv,
                                                    &find_query,
                                                    find_use_regex,
                                                    find_whole_word,
                                                    find_case_insensitive,
                                                    sel,
                                                );
                                                find_current = find_match_at_or_after(
                                                    &find_matches,
                                                    find_anchor.0,
                                                    find_anchor.1,
                                                );
                                                if let Some(i) = find_current {
                                                    select_find_match(
                                                        dv,
                                                        find_matches[i],
                                                        replace_active,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                        match key.as_str() {
                            "escape" => {
                                find_active = false;
                                replace_active = false;
                                find_focus_on_replace = false;
                                // Free the resident full-document search subject
                                // now that the find bar is closed; it is rebuilt
                                // lazily on the next search.
                                if let Some(doc) = docs.get(active_tab) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let _ = buffer::with_buffer_mut(buf_id, |b| {
                                            b.search_cache = None;
                                            Ok(())
                                        });
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "tab" if replace_active => {
                                find_focus_on_replace = !find_focus_on_replace;
                                redraw = true;
                                continue;
                            }
                            "f3" => {
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    if !find_matches.is_empty() {
                                        let idx = if mods.shift {
                                            let (al, ac) = doc_anchor(dv);
                                            find_match_before(&find_matches, al, ac)
                                                .unwrap_or(find_matches.len() - 1)
                                        } else {
                                            let (cl, cc) = doc_cursor(dv);
                                            find_match_at_or_after(&find_matches, cl, cc)
                                                .unwrap_or(0)
                                        };
                                        find_current = Some(idx);
                                        select_find_match(dv, find_matches[idx], replace_active);
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "return" | "keypad enter"
                                if mods.ctrl && mods.shift && replace_active =>
                            {
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    let mut count = 0usize;
                                    loop {
                                        let sel = if find_in_selection {
                                            find_selection_range
                                        } else {
                                            None
                                        };
                                        let matches = compute_find_matches_filtered(
                                            dv,
                                            &find_query,
                                            find_use_regex,
                                            find_whole_word,
                                            find_case_insensitive,
                                            sel,
                                        );
                                        if matches.is_empty() {
                                            break;
                                        }
                                        select_find_match(dv, matches[0], replace_active);
                                        replace_current_match(dv, &find_query, &replace_query);
                                        count += 1;
                                        if count > 100_000 {
                                            break;
                                        }
                                    }
                                    find_matches.clear();
                                    find_current = None;
                                    info_message = Some((
                                        format!("Replaced {count} occurrence(s)"),
                                        Instant::now(),
                                    ));
                                }
                                redraw = true;
                                continue;
                            }
                            "return" | "keypad enter"
                                if mods.ctrl && !mods.shift && replace_active =>
                            {
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    replace_current_match(dv, &find_query, &replace_query);
                                    let sel = if find_in_selection {
                                        find_selection_range
                                    } else {
                                        None
                                    };
                                    find_matches = compute_find_matches_filtered(
                                        dv,
                                        &find_query,
                                        find_use_regex,
                                        find_whole_word,
                                        find_case_insensitive,
                                        sel,
                                    );
                                    if !find_matches.is_empty() {
                                        let (cl, cc) = doc_cursor(dv);
                                        let idx = find_match_at_or_after(&find_matches, cl, cc)
                                            .unwrap_or(0);
                                        find_current = Some(idx);
                                        select_find_match(dv, find_matches[idx], replace_active);
                                    } else {
                                        find_current = None;
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "r" if mods.alt && replace_active => {
                                // Alt+R: replace current match (NoteSquirrel parity).
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    replace_current_match(dv, &find_query, &replace_query);
                                    let sel = if find_in_selection {
                                        find_selection_range
                                    } else {
                                        None
                                    };
                                    find_matches = compute_find_matches_filtered(
                                        dv,
                                        &find_query,
                                        find_use_regex,
                                        find_whole_word,
                                        find_case_insensitive,
                                        sel,
                                    );
                                    if !find_matches.is_empty() {
                                        let (cl, cc) = doc_cursor(dv);
                                        let idx = find_match_at_or_after(&find_matches, cl, cc)
                                            .unwrap_or(0);
                                        find_current = Some(idx);
                                        select_find_match(dv, find_matches[idx], replace_active);
                                    } else {
                                        find_current = None;
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "a" if mods.alt && replace_active => {
                                // Alt+A: replace all matches (NoteSquirrel parity).
                                // Drives `replace_current_match` in a loop
                                // since JereIDE doesn't have a separate
                                // bulk-replace primitive for the in-buffer
                                // find bar.
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    let mut count = 0usize;
                                    loop {
                                        let sel = if find_in_selection {
                                            find_selection_range
                                        } else {
                                            None
                                        };
                                        let matches = compute_find_matches_filtered(
                                            dv,
                                            &find_query,
                                            find_use_regex,
                                            find_whole_word,
                                            find_case_insensitive,
                                            sel,
                                        );
                                        if matches.is_empty() {
                                            break;
                                        }
                                        select_find_match(dv, matches[0], replace_active);
                                        replace_current_match(dv, &find_query, &replace_query);
                                        count += 1;
                                        if count > 100_000 {
                                            break;
                                        }
                                    }
                                    find_matches.clear();
                                    find_current = None;
                                    info_message = Some((
                                        format!("Replaced {count} occurrence(s)"),
                                        Instant::now(),
                                    ));
                                }
                                redraw = true;
                                continue;
                            }
                            "return" | "keypad enter" => {
                                // Shift+Enter = previous, Enter = next.
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    let dv = &mut doc.view;
                                    if !find_matches.is_empty() {
                                        let idx = if mods.shift {
                                            let (al, ac) = doc_anchor(dv);
                                            find_match_before(&find_matches, al, ac)
                                                .unwrap_or(find_matches.len() - 1)
                                        } else {
                                            let (cl, cc) = doc_cursor(dv);
                                            find_match_at_or_after(&find_matches, cl, cc)
                                                .unwrap_or(0)
                                        };
                                        find_current = Some(idx);
                                        select_find_match(dv, find_matches[idx], replace_active);
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            "backspace" => {
                                if find_focus_on_replace {
                                    if !replace_query.is_empty() {
                                        replace_history.record(
                                            &replace_query,
                                            replace_query.len(),
                                            FieldEdit::Delete,
                                            buffer::now_secs(),
                                        );
                                    }
                                    replace_query.pop();
                                } else {
                                    if !find_query.is_empty() {
                                        find_history.record(
                                            &find_query,
                                            find_query.len(),
                                            FieldEdit::Delete,
                                            buffer::now_secs(),
                                        );
                                    }
                                    find_query.pop();
                                    if let Some(doc) = docs.get_mut(active_tab) {
                                        let dv = &mut doc.view;
                                        let sel = if find_in_selection {
                                            find_selection_range
                                        } else {
                                            None
                                        };
                                        find_matches = compute_find_matches_filtered(
                                            dv,
                                            &find_query,
                                            find_use_regex,
                                            find_whole_word,
                                            find_case_insensitive,
                                            sel,
                                        );
                                        find_current = find_match_at_or_after(
                                            &find_matches,
                                            find_anchor.0,
                                            find_anchor.1,
                                        );
                                        if let Some(i) = find_current {
                                            select_find_match(dv, find_matches[i], replace_active);
                                        }
                                    }
                                }
                                redraw = true;
                                continue;
                            }
                            _ => {
                                // Unhandled keys (Home, End, arrow keys, page
                                // up/down, etc.) fall through to the main keymap
                                // dispatch so doc navigation keeps working while
                                // the find bar is visible. Bare letters reach
                                // the keymap with no binding and become no-ops;
                                // the paired TextInput event still appends them
                                // to the find query input below.
                            }
                        }
                    }

                    // Insert key toggles overwrite mode.
                    if key == "insert" && !mods.ctrl && !mods.alt && !mods.shift {
                        overwrite_mode = !overwrite_mode;
                        redraw = true;
                        continue;
                    }

                    // Direct Ctrl+=/- handling (SDL key names vary by platform).
                    if mods.ctrl && !mods.alt && !mods.shift {
                        let scale_cmd = match key.as_str() {
                            "=" | "+" | "equals" | "keypad +" => Some("scale:increase"),
                            "-" | "minus" | "keypad -" => Some("scale:decrease"),
                            "0" | "keypad 0" => Some("scale:reset"),
                            _ => None,
                        };
                        if let Some(cmd) = scale_cmd {
                            let current_logical = config.fonts.ui.size as i32;
                            let new_logical = match cmd {
                                "scale:increase" => (current_logical + 1).min(48),
                                "scale:decrease" => (current_logical - 1).max(6),
                                _ => 15, // reset
                            };
                            let new_size = new_logical as f32 * display_scale as f32;
                            let mut new_config = config.clone();
                            new_config.fonts.ui.size = new_logical as u32;
                            new_config.fonts.code.size = new_logical as u32;
                            if let Ok(new_ctx) = load_fonts(&new_config) {
                                config = new_config.clone();
                                draw_ctx = new_ctx;
                                style = build_style(&config, &draw_ctx);
                                style.scale = display_scale;
                                style.padding_x *= display_scale;
                                style.padding_y *= display_scale;
                                style.divider_size = (style.divider_size * display_scale).ceil();
                                style.scrollbar_size *= display_scale;
                                style.caret_width = (style.caret_width * display_scale).ceil();
                                style.tab_width *= display_scale;
                                let tp = Path::new(datadir)
                                    .join("assets")
                                    .join("themes")
                                    .join(format!("{}.json", config.theme));
                                if let Ok(palette) =
                                    crate::editor::style::load_theme_palette(&tp.to_string_lossy())
                                {
                                    apply_theme_to_style(&mut style, &palette);
                                }
                                crate::editor::style_ctx::set_current_style(style.clone());
                                let _ = crate::editor::storage::save_text(
                                    userdir_path,
                                    "session",
                                    "font_size",
                                    &new_size.to_string(),
                                );
                            }
                            redraw = true;
                            continue;
                        }
                    }

                    // Direct Ctrl+` handling for terminal toggle.
                    if subsystems.has_terminal() {
                        if mods.ctrl
                            && !mods.alt
                            && !mods.shift
                            && (key == "`" || key == "grave" || key == "backquote")
                        {
                            terminal.visible = !terminal.visible;
                            if terminal.visible && terminal.terminals.is_empty() {
                                let active_doc_path =
                                    docs.get(active_tab).map(|d| d.path.as_str()).unwrap_or("");
                                let cwd = crate::editor::terminal_panel::resolve_terminal_cwd(
                                    active_doc_path,
                                    &project_root,
                                );
                                if terminal.spawn(&cwd) {
                                    let n = terminal.terminals.len();
                                    let cd_payload =
                                        crate::editor::terminal_panel::terminal_cd_payload(&cwd);
                                    if let Some(t) = terminal.active_terminal() {
                                        t.title =
                                            crate::editor::terminal_panel::terminal_title(n, &cwd);
                                        let _ = t.inner.write(cd_payload.as_bytes());
                                    }
                                }
                            }
                            terminal.focused = terminal.visible;
                            redraw = true;
                            continue;
                        }

                        // Direct Ctrl+Shift+T for new terminal.
                        if mods.ctrl && mods.shift && !mods.alt && key == "t" {
                            let active_doc_path =
                                docs.get(active_tab).map(|d| d.path.as_str()).unwrap_or("");
                            let cwd = crate::editor::terminal_panel::resolve_terminal_cwd(
                                active_doc_path,
                                &project_root,
                            );
                            let ok = terminal.spawn(&cwd);
                            if ok {
                                let n = terminal.terminals.len();
                                let cd_payload =
                                    crate::editor::terminal_panel::terminal_cd_payload(&cwd);
                                if let Some(t) = terminal.active_terminal() {
                                    t.title =
                                        crate::editor::terminal_panel::terminal_title(n, &cwd);
                                    let _ = t.inner.write(cd_payload.as_bytes());
                                }
                            }
                            redraw = true;
                            continue;
                        }
                    }

                    if let Some(cmds) = keymap.on_key_pressed(key, mods) {
                        for cmd in Vec::from(cmds) {
                            {
                                let cmd: String = cmd;
                                // Close any active palette or cmdview before dispatching.
                                if palette_active {
                                    palette_active = false;
                                }
                                if cmdview_active {
                                    cmdview_active = false;
                                }
                                if theme_picker_active {
                                    if let Some(orig) = theme_picker_original_style.take() {
                                        style = orig;
                                        current_theme_idx = theme_picker_original_idx;
                                    }
                                    theme_picker_active = false;
                                }
                                include!("commands_dispatch.rs");
                            }
                        }
                    }
                    redraw = true;
                }
                EditorEvent::TextInput(text) => {
                    cursor_blink_reset = Instant::now();
                    // The KeyDown handler already consumed this key
                    // (e.g. Y / N resolving a nag); drop the paired
                    // TextInput so it can't land in the document.
                    if eat_next_text_input {
                        eat_next_text_input = false;
                        redraw = true;
                        continue;
                    }
                    // Block text input while *any* nag is active —
                    // characters typed before the user presses Y / N
                    // must not leak into the doc.
                    if !matches!(nag, Nag::None) {
                        cmdview_active = false;
                        palette_active = false;
                        redraw = true;
                        continue;
                    }
                    // Route typing into the sidebar search when focused.

                    // Forward text to terminal when focused.
                    if subsystems.has_terminal() && terminal.visible && terminal.focused {
                        if let Some(inst) = terminal.active_terminal() {
                            let _ = inst.inner.write(text.as_bytes());
                            inst.scrollback = 0.0;
                            inst.scrollback_target = 0.0;
                        }
                        redraw = true;
                        continue;
                    }
                    // Route typed characters into the inline new-file input.
                    if sidebar_new_file_dir.is_some() {
                        sidebar_new_file_history.record(
                            &sidebar_new_file_name,
                            sidebar_new_file_cursor,
                            FieldEdit::Insert,
                            buffer::now_secs(),
                        );
                        sidebar_new_file_name.insert_str(sidebar_new_file_cursor, text);
                        sidebar_new_file_cursor += text.len();
                        redraw = true;
                        continue;
                    }
                    if cmdview_active
                        && (subsystems.has_picker()
                            || cmdview_mode == CmdViewMode::SaveAs
                            || cmdview_mode == CmdViewMode::OpenFile
                            || cmdview_mode == CmdViewMode::OpenRecent
                            || cmdview_mode == CmdViewMode::Rename)
                    {
                        let prev_text = cmdview_text.clone();
                        cmdview_history.record(
                            &cmdview_text,
                            cmdview_cursor,
                            FieldEdit::Insert,
                            buffer::now_secs(),
                        );
                        // Insert at the caret rather than appending so left/right/home/end
                        // editing is preserved while typing.
                        cmdview_text.insert_str(cmdview_cursor, text);
                        cmdview_cursor += text.len();
                        let dirs_only = cmdview_mode == CmdViewMode::OpenFolder;
                        if cmdview_mode == CmdViewMode::OpenRecent {
                            let query = cmdview_text.to_lowercase();
                            let mut combined: Vec<String> = Vec::new();
                            if !false {
                                for p in &recent_projects {
                                    if !combined.contains(p) {
                                        combined.push(p.clone());
                                    }
                                }
                            }
                            for p in &recent_files {
                                if !combined.contains(p) {
                                    combined.push(p.clone());
                                }
                            }
                            cmdview_suggestions = if query.is_empty() {
                                combined
                            } else {
                                combined
                                    .into_iter()
                                    .filter(|p| p.to_lowercase().contains(&query))
                                    .collect()
                            };
                        } else if cmdview_text.is_empty() {
                            cmdview_suggestions = if dirs_only {
                                recent_projects.clone()
                            } else {
                                recent_files.clone()
                            };
                        } else {
                            cmdview_suggestions =
                                path_suggest(&cmdview_text, &project_root, dirs_only);
                        }
                        cmdview_selected = 0;
                        // Typeahead: auto-fill when exactly one suggestion matches.
                        // Disabled for SaveAs -- suggestions are shown as options
                        // but must not overwrite what the user is typing. Also
                        // disabled in OpenRecent where suggestions are filtered
                        // by substring, not prefix.
                        if cmdview_mode != CmdViewMode::SaveAs
                            && cmdview_mode != CmdViewMode::OpenRecent
                            && cmdview_mode != CmdViewMode::Rename
                            && cmdview_suggestions.len() == 1
                            && cmdview_cursor == cmdview_text.len()
                            && cmdview_text.len() > prev_text.len()
                            && !cmdview_text.ends_with('/')
                        {
                            let suggestion = &cmdview_suggestions[0];
                            if suggestion.starts_with(&cmdview_text) {
                                cmdview_text = suggestion.clone();
                                cmdview_cursor = cmdview_text.len();
                            }
                        }
                        redraw = true;
                        continue;
                    }
                    if subsystems.has_find_in_files() && project_search_active {
                        project_search_history.record(
                            &project_search_query,
                            project_search_query.len(),
                            FieldEdit::Insert,
                            buffer::now_secs(),
                        );
                        project_search_query.push_str(text);
                        project_search_results = project_search::run_project_search(
                            &project_search_query,
                            &project_root,
                            project_use_regex,
                            project_whole_word,
                            project_case_insensitive,
                        );
                        project_search_selected = 0;
                        redraw = true;
                        continue;
                    }
                    if subsystems.has_find_in_files() && project_replace_active {
                        if project_replace_focus_on_replace {
                            project_replace_with_history.record(
                                &project_replace_with,
                                project_replace_with.len(),
                                FieldEdit::Insert,
                                buffer::now_secs(),
                            );
                            project_replace_with.push_str(text);
                        } else {
                            project_replace_search_history.record(
                                &project_replace_search,
                                project_replace_search.len(),
                                FieldEdit::Insert,
                                buffer::now_secs(),
                            );
                            project_replace_search.push_str(text);
                            project_replace_results = project_search::run_project_search(
                                &project_replace_search,
                                &project_root,
                                project_use_regex,
                                project_whole_word,
                                project_case_insensitive,
                            );
                            project_replace_selected = 0;
                        }
                        redraw = true;
                        continue;
                    }
                    if palette_active {
                        palette_history.record(
                            &palette_query,
                            palette_query.len(),
                            FieldEdit::Insert,
                            buffer::now_secs(),
                        );
                        palette_query.push_str(text);
                        palette_results = fuzzy_filter_commands(&palette_query, &all_commands);
                        palette_selected = 0;
                        redraw = true;
                        continue;
                    }
                    if theme_picker_active {
                        theme_picker_query.push_str(text);
                        theme_picker_results = if theme_picker_query.is_empty() {
                            available_themes.iter().map(|t| (t.clone(), t.clone())).collect()
                        } else {
                            let q = theme_picker_query.to_lowercase();
                            available_themes.iter().filter(|t| {
                                t.to_lowercase().contains(&q)
                            }).map(|t| (t.clone(), t.clone())).collect()
                        };
                        theme_picker_selected = 0;
                        redraw = true;
                        continue;
                    }
                    if nag.is_unsaved() {
                        cmdview_active = false;
                        palette_active = false;
                        redraw = true;
                        continue;
                    }
                    if find_active {
                        if find_focus_on_replace {
                            replace_history.record(
                                &replace_query,
                                replace_query.len(),
                                FieldEdit::Insert,
                                buffer::now_secs(),
                            );
                            replace_query.push_str(text);
                        } else {
                            find_history.record(
                                &find_query,
                                find_query.len(),
                                FieldEdit::Insert,
                                buffer::now_secs(),
                            );
                            find_query.push_str(text);
                            if let Some(doc) = docs.get_mut(active_tab) {
                                let dv = &mut doc.view;
                                let sel = if find_in_selection {
                                    find_selection_range
                                } else {
                                    None
                                };
                                find_matches = compute_find_matches_filtered(
                                    dv,
                                    &find_query,
                                    find_use_regex,
                                    find_whole_word,
                                    find_case_insensitive,
                                    sel,
                                );
                                find_current = find_match_at_or_after(
                                    &find_matches,
                                    find_anchor.0,
                                    find_anchor.1,
                                );
                                if let Some(i) = find_current {
                                    select_find_match(dv, find_matches[i], replace_active);
                                }
                            }
                        }
                        redraw = true;
                        continue;
                    }
                    if let Some(doc) = docs.get_mut(active_tab) {
                        let dv = &mut doc.view;
                        if let Some(buf_id) = dv.buffer_id {
                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                let is_single_char = text.chars().count() == 1;
                                let has_sel = b.selections.len() >= 4
                                    && (b.selections[0] != b.selections[2]
                                        || b.selections[1] != b.selections[3]);
                                if is_single_char && !has_sel {
                                    let line = *b.selections.first().unwrap_or(&1);
                                    let col = *b.selections.get(1).unwrap_or(&1);
                                    buffer::push_undo_mergeable(b, line, col, false);
                                } else {
                                    buffer::push_undo(b);
                                }
                                // Typing over an active selection replaces it. Only the
                                // single-cursor case is handled here; multi-cursor selection
                                // replacement would need per-cursor reverse-order deletion.
                                if has_sel && buffer::cursor_count(b) == 1 {
                                    buffer::delete_selection(b);
                                }
                                // Collect cursor positions, sorted bottom-to-top so
                                // insertions don't shift earlier cursor positions.
                                let n = buffer::cursor_count(b);
                                let mut cursor_positions: Vec<(usize, usize, usize)> = (0..n)
                                    .map(|i| {
                                        let base = i * 4;
                                        (i, b.selections[base + 2], b.selections[base + 3])
                                    })
                                    .collect();
                                cursor_positions
                                    .sort_by(|a, b_pos| b_pos.1.cmp(&a.1).then(b_pos.2.cmp(&a.2)));
                                let text_len = text.chars().count();
                                for &(idx, cline, ccol) in &cursor_positions {
                                    let _ = idx;
                                    if cline <= b.lines.len() {
                                        let l = &mut b.lines[cline - 1];
                                        let byte_pos = char_to_byte(l, ccol - 1);
                                        // In overwrite mode, delete the char at cursor before inserting.
                                        if overwrite_mode {
                                            let trimmed = l.trim_end_matches('\n');
                                            if byte_pos < trimmed.len() {
                                                let end = l
                                                    .char_indices()
                                                    .nth(ccol)
                                                    .map(|(i, _)| i)
                                                    .unwrap_or(trimmed.len());
                                                l.replace_range(byte_pos..end, "");
                                            }
                                        }
                                        let l = &mut b.lines[cline - 1];
                                        let byte_pos = char_to_byte(l, ccol - 1);
                                        l.insert_str(byte_pos, text);
                                    }
                                }
                                // Update all cursor positions after insertion.
                                // Re-sort top-to-bottom to adjust for same-line shifts.
                                cursor_positions
                                    .sort_by(|a, b_pos| a.1.cmp(&b_pos.1).then(a.2.cmp(&b_pos.2)));
                                let mut col_offset_on_line: Vec<(usize, usize)> = Vec::new();
                                for &(idx, cline, ccol) in &cursor_positions {
                                    let extra: usize = col_offset_on_line
                                        .iter()
                                        .filter(|(l, _)| *l == cline)
                                        .map(|(_, o)| o)
                                        .sum();
                                    let new_col = ccol + extra + text_len;
                                    let base = idx * 4;
                                    b.selections[base] = cline;
                                    b.selections[base + 1] = new_col;
                                    b.selections[base + 2] = cline;
                                    b.selections[base + 3] = new_col;
                                    col_offset_on_line.push((cline, text_len));
                                }
                                Ok(())
                            });
                        }
                        if subsystems.has_lsp() {
                            // Buffer-mutation marking happens generically in
                            // the per-frame change_id watcher; nothing to do
                            // here on the typing path beyond completion
                            // triggers below.
                            //
                            // Trigger LSP completion after trigger characters.
                            let trigger = text == "." || text == ":" || text == "(";
                            let word_char = text
                                .chars()
                                .next()
                                .map(|c| c.is_alphanumeric() || c == '_')
                                .unwrap_or(false);
                            if (trigger || word_char)
                                && lsp_state.transport_id.is_some()
                                && lsp_state.initialized
                            {
                                if let Some(doc) = docs.get(active_tab) {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        if !doc.path.is_empty() {
                                            let tid = lsp_state.transport_id.unwrap();
                                            let (cl, cc) = buffer::with_buffer(buf_id, |b| {
                                                let l = *b.selections.get(2).unwrap_or(&1);
                                                let c = *b.selections.get(3).unwrap_or(&1);
                                                Ok((l, c))
                                            })
                                            .unwrap_or((1, 1));
                                            let uri = path_to_uri(&doc.path);
                                            let req_id = lsp_state.next_id();
                                            lsp_state.pending_requests.insert(
                                                req_id,
                                                "textDocument/completion".to_string(),
                                            );
                                            let _ = lsp::send_message(
                                                tid,
                                                &lsp_completion_request(
                                                    req_id,
                                                    &uri,
                                                    cl - 1,
                                                    cc - 1,
                                                ),
                                            );
                                            completion.line = cl;
                                            completion.col = cc;
                                            completion.latest_request_id = req_id;
                                        }
                                    }
                                }
                            }
                            // Trigger signature help after '(' or ','; hide on ')'.
                            if text == ")" {
                                signature_help.hide();
                            } else if (text == "(" || text == ",")
                                && lsp_state.transport_id.is_some()
                                && lsp_state.initialized
                                && let Some(doc) = docs.get(active_tab)
                                && let Some(buf_id) = doc.view.buffer_id
                                && !doc.path.is_empty()
                            {
                                let tid = lsp_state.transport_id.unwrap();
                                let (cl, cc) = buffer::with_buffer(buf_id, |b| {
                                    Ok((
                                        *b.selections.get(2).unwrap_or(&1),
                                        *b.selections.get(3).unwrap_or(&1),
                                    ))
                                })
                                .unwrap_or((1, 1));
                                let uri = path_to_uri(&doc.path);
                                let req_id = lsp_state.next_id();
                                lsp_state
                                    .pending_requests
                                    .insert(req_id, "textDocument/signatureHelp".to_string());
                                let _ = lsp::send_message(
                                    tid,
                                    &lsp_signature_help_request(req_id, &uri, cl - 1, cc - 1),
                                );
                                signature_help.line = cl;
                                signature_help.col = cc;
                            }
                        }
                        // Document-word autocomplete: instant, no LSP dependency.
                        // Fires on every word character typed when LSP isn't
                        // handling it.
                        let dwp_word_char = text
                            .chars()
                            .next()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false);
                        if dwp_word_char {
                            let lsp_handles = subsystems.has_lsp()
                                && lsp_state.transport_id.is_some()
                                && lsp_state.initialized;
                            if !lsp_handles {
                                if word_index.dirty {
                                    if let Some(buf_id) =
                                        docs.get(active_tab).and_then(|d| d.view.buffer_id)
                                    {
                                        let _ = buffer::with_buffer(buf_id, |b| {
                                            word_index.rebuild(&b.lines);
                                            Ok(())
                                        });
                                    }
                                }
                                if let Some(buf_id) =
                                    docs.get(active_tab).and_then(|d| d.view.buffer_id)
                                {
                                    let (cl, cc, prefix) = buffer::with_buffer(buf_id, |b| {
                                        let l = *b.selections.get(2).unwrap_or(&1);
                                        let c = *b.selections.get(3).unwrap_or(&1);
                                        let line =
                                            b.lines.get(l - 1).map(String::as_str).unwrap_or("");
                                        let prefix_chars: Vec<char> = line.chars().collect();
                                        let col = (c - 1).min(prefix_chars.len());
                                        let mut start = col;
                                        while start > 0 {
                                            if prefix_chars[start - 1].is_alphanumeric()
                                                || prefix_chars[start - 1] == '_'
                                            {
                                                start -= 1;
                                            } else {
                                                break;
                                            }
                                        }
                                        Ok((
                                            l,
                                            c,
                                            prefix_chars[start..col].iter().collect::<String>(),
                                        ))
                                    })
                                    .unwrap_or((1, 1, String::new()));
                                    if !prefix.is_empty() {
                                        let items = word_index.query(&prefix, 20);
                                        if !items.is_empty() {
                                            completion.items = items;
                                            completion.selected = 0;
                                            completion.scroll_offset = 0;
                                            completion.line = cl;
                                            completion.col = cc;
                                            completion.visible = true;
                                        } else if !completion.visible {
                                            completion.hide();
                                        }
                                    } else {
                                        completion.hide();
                                    }
                                }
                            }
                        }
                    }
                    redraw = true;
                }
                EditorEvent::MousePressed {
                    button,
                    x,
                    y,
                    clicks,
                    modifiers,
                    ..
                } => {
                    cursor_blink_reset = Instant::now();
                    // Any mouse click cancels pending scroll animation so the
                    // view never jumps unexpectedly.
                    if let Some(doc) = docs.get_mut(active_tab) {
                        doc.view.target_scroll_y = doc.view.scroll_y;
                    }
                    // Nag bar button click handling.
                    if let Nag::UnsavedChanges {
                        message,
                        tab_to_close,
                    } = &nag
                    {
                        if *button == MouseButton::Left {
                            let message = message.clone();
                            let tab_to_close = *tab_to_close;
                            use crate::editor::view::DrawContext as _;
                            let bar_h = style.font_height + style.padding_y * 2.0;
                            if *y < bar_h {
                                let msg_w = draw_ctx.font_width(style.font, &message);
                                let btn_pad = style.padding_x;
                                let mut bx = style.padding_x + msg_w + btn_pad * 2.0;
                                for (i, label) in ["Yes", "No"].iter().enumerate() {
                                    let lw = draw_ctx.font_width(style.font, label) + btn_pad * 2.0;
                                    if *x >= bx && *x <= bx + lw {
                                        if i == 0 {
                                            // Yes: discard unsaved changes and proceed.
                                            if let Some(idx) = tab_to_close {
                                                if let Some(d) = docs.get(idx) {
                                                    autoreload.unwatch(&d.path);
                                                }
                                                docs.remove(idx);
                                                if active_tab >= docs.len() && !docs.is_empty() {
                                                    active_tab = docs.len() - 1;
                                                }
                                            } else {
                                                quit = true;
                                            }
                                        }
                                        // No (i == 1): just dismiss the nag.
                                        nag = Nag::None;
                                        redraw = true;
                                        continue;
                                    }
                                    bx += lw + btn_pad;
                                }
                            }
                        }
                    }

                    // Context menu: left-click outside dismisses, right-click shows.
                    if context_menu.visible && *button == MouseButton::Left {
                        let (menu_x, menu_y, menu_w, menu_h) = context_menu.render_rect;
                        let item_h = style.font_height + style.padding_y;
                        if menu_h > 0.0
                            && *x >= menu_x
                            && *x <= menu_x + menu_w
                            && *y >= menu_y
                            && *y <= menu_y + menu_h
                        {
                            let idx =
                                ((*y - menu_y - style.padding_y / 2.0) / item_h).floor() as usize;
                            if let Some(item) = context_menu.items.get(idx) {
                                if let Some(ref cmd) = item.command {
                                    let cmd = cmd.clone();
                                    context_menu.hide();
                                    if cmd == "sidebar:new" {
                                        if let Some((path, is_dir)) = sidebar_menu_target.take() {
                                            let dir = if is_dir {
                                                path
                                            } else {
                                                std::path::Path::new(&path)
                                                    .parent()
                                                    .map(|p| p.to_string_lossy().to_string())
                                                    .unwrap_or_else(|| project_root.clone())
                                            };
                                            // Expand the target directory in the sidebar if
                                            // it isn't already so the inline input is visible.
                                            if let Some(dir_idx) = sidebar_entries
                                                .iter()
                                                .position(|e| e.is_dir && e.path == dir)
                                            {
                                                if !sidebar_entries[dir_idx].expanded {
                                                    sidebar_entries[dir_idx].expanded = true;
                                                    let depth = sidebar_entries[dir_idx].depth;
                                                    let children = scan_directory(
                                                        &dir,
                                                        depth + 1,
                                                        sidebar_show_hidden,
                                                    );
                                                    for (i, child) in
                                                        children.into_iter().enumerate()
                                                    {
                                                        sidebar_entries
                                                            .insert(dir_idx + 1 + i, child);
                                                    }
                                                    sidebar_watcher.watch_dir(&dir);
                                                }
                                            }
                                            sidebar_new_file_dir = Some(dir);
                                            sidebar_new_file_name.clear();
                                            sidebar_new_file_cursor = 0;
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "sidebar:rename" {
                                        if let Some((path, _is_dir)) = sidebar_menu_target.take() {
                                            rename_source = path.clone();
                                            cmdview_active = true;
                                            cmdview_mode = CmdViewMode::Rename;
                                            cmdview_text = path;
                                            cmdview_cursor = cmdview_text.len();
                                            cmdview_label = "Rename:".to_string();
                                            cmdview_suggestions = Vec::new();
                                            cmdview_selected = 0;
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "sidebar:delete" {
                                        if let Some((path, is_dir)) = sidebar_menu_target.take() {
                                            if !is_dir {
                                                nag = Nag::DeleteFile { path };
                                            }
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "sidebar:copy-path" {
                                        if let Some((path, _)) = sidebar_menu_target.take() {
                                            crate::window::set_clipboard_text(&path);
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "sidebar:copy-relative-path" {
                                        if let Some((path, _)) = sidebar_menu_target.take() {
                                            let rel = std::path::Path::new(&path)
                                                .strip_prefix(&project_root)
                                                .map(|p| p.to_string_lossy().into_owned())
                                                .unwrap_or_else(|_| path.clone());
                                            crate::window::set_clipboard_text(&rel);
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd.starts_with("test:") {
                                        let cmd: String = cmd;
                                        include!("commands_dispatch.rs");
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "tab:copy-path" {
                                        if let Some(target) = tab_menu_target.take() {
                                            if let Some(d) = docs.get(target) {
                                                if !d.path.is_empty() {
                                                    crate::window::set_clipboard_text(&d.path);
                                                }
                                            }
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd == "tab:copy-relative-path" {
                                        if let Some(target) = tab_menu_target.take() {
                                            if let Some(d) = docs.get(target) {
                                                if !d.path.is_empty() {
                                                    let rel = std::path::Path::new(&d.path)
                                                        .strip_prefix(&project_root)
                                                        .map(|p| p.to_string_lossy().into_owned())
                                                        .unwrap_or_else(|_| d.path.clone());
                                                    crate::window::set_clipboard_text(&rel);
                                                }
                                            }
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    if cmd.starts_with("tab:close") {
                                        if let Some(target) = tab_menu_target.take() {
                                            // `indices` is built in reverse so
                                            // removing by index stays valid as the
                                            // list shrinks.
                                            let total = docs.len();
                                            let indices: Vec<usize> = match cmd.as_str() {
                                                "tab:close" => {
                                                    if target < total {
                                                        vec![target]
                                                    } else {
                                                        vec![]
                                                    }
                                                }
                                                "tab:close-right" => {
                                                    ((target + 1)..total).rev().collect()
                                                }
                                                "tab:close-left" => (0..target).rev().collect(),
                                                "tab:close-all" => (0..total).rev().collect(),
                                                _ => vec![],
                                            };
                                            // If any targeted doc is modified, nag
                                            // on the first modified one and skip
                                            // the rest — matches the close-button
                                            // safety net so we don't silently drop
                                            // unsaved buffers in a batch op.
                                            let first_mod =
                                                indices.iter().rev().copied().find(|&i| {
                                                    docs.get(i).is_some_and(doc_is_modified)
                                                });
                                            if let Some(i) = first_mod {
                                                let name = docs[i].name.clone();
                                                nag = Nag::UnsavedChanges {
                                                    message: nag_msg_close(&name),
                                                    tab_to_close: Some(i),
                                                };
                                            } else {
                                                for i in indices {
                                                    if let Some(d) = docs.get(i) {
                                                        autoreload.unwatch(&d.path);
                                                        if !d.path.is_empty() {
                                                            closed_tabs.retain(|p| p != &d.path);
                                                            closed_tabs.push(d.path.clone());
                                                            if closed_tabs.len() > 25 {
                                                                closed_tabs.remove(0);
                                                            }
                                                        }
                                                    }
                                                    docs.remove(i);
                                                }
                                                if active_tab >= docs.len() && !docs.is_empty() {
                                                    active_tab = docs.len() - 1;
                                                } else if docs.is_empty() {
                                                    active_tab = 0;
                                                } else if cmd == "tab:close-left" {
                                                    // The active tab's index
                                                    // shifted by the number of
                                                    // docs removed from the left.
                                                    active_tab = active_tab.saturating_sub(target);
                                                }
                                            }
                                        }
                                        redraw = true;
                                        continue;
                                    }
                                    {
                                        include!("commands_dispatch.rs");
                                    }
                                    redraw = true;
                                    continue;
                                }
                            }
                        }
                        context_menu.hide();
                        redraw = true;
                        continue;
                    }

                    if *button == MouseButton::Right {
                        // Right-click on a tab: show the tab context menu (Close /
                        // Close others left|right / Close all). Clicks on the
                        // dropdown button or empty tab-bar space are swallowed so
                        // the doc Cut/Copy/Paste menu doesn't spawn off-screen at
                        // the far right of the window.
                        let tab_h_rc = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        if *y < tab_h_rc {
                            use crate::editor::view::DrawContext as _;
                            let sidebar_w_tab = if subsystems.has_sidebar() && sidebar_visible {
                                sidebar_width
                            } else {
                                0.0
                            };
                            let (ww_tr, wh_tr, _, _) = crate::window::get_window_size();
                            let win_w_tr = ww_tr as f64;
                            let win_h_tr = wh_tr as f64;
                            let close_btn_w =
                                draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                            let dropdown_btn_w = (style.font_height + style.padding_x * 2.0).ceil();
                            let avail_full = (win_w_tr - sidebar_w_tab).max(0.0);
                            let mut full_total = 0.0_f64;
                            for doc in docs.iter() {
                                let label = if doc_is_modified(doc) {
                                    format!("*{}", doc.name)
                                } else {
                                    doc.name.clone()
                                };
                                full_total += draw_ctx.font_width(style.font, &label)
                                    + style.padding_x * 2.0
                                    + close_btn_w
                                    + style.divider_size;
                            }
                            let tabs_overflow = full_total > avail_full;
                            let tabs_right_limit = if tabs_overflow {
                                (win_w_tr - dropdown_btn_w).max(sidebar_w_tab)
                            } else {
                                win_w_tr
                            };

                            // Walk tabs in the same order / widths as the draw
                            // pass, find the one under the click.
                            let mut tx = sidebar_w_tab;
                            let mut hit: Option<usize> = None;
                            for (i, doc) in docs.iter().enumerate() {
                                let display_label = if tabs_overflow {
                                    let base = truncate_tab_name(&doc.name, 10);
                                    if doc_is_modified(doc) {
                                        format!("*{base}")
                                    } else {
                                        base
                                    }
                                } else if doc_is_modified(doc) {
                                    format!("*{}", doc.name)
                                } else {
                                    doc.name.clone()
                                };
                                let tw = draw_ctx.font_width(style.font, &display_label)
                                    + style.padding_x * 2.0
                                    + close_btn_w
                                    + style.divider_size;
                                let hit_right = (tx + tw).min(tabs_right_limit);
                                if *x >= tx && *x < hit_right {
                                    hit = Some(i);
                                    break;
                                }
                                tx += tw;
                                if tx >= tabs_right_limit {
                                    break;
                                }
                            }
                            if let Some(i) = hit {
                                tab_menu_target = Some(i);
                                let total = docs.len();
                                let mut items = vec![MenuItem {
                                    text: "Close".into(),
                                    info: None,
                                    command: Some("tab:close".into()),
                                    separator: false,
                                }];
                                if i + 1 < total {
                                    items.push(MenuItem {
                                        text: "Close All to the Right".into(),
                                        info: None,
                                        command: Some("tab:close-right".into()),
                                        separator: false,
                                    });
                                }
                                if i > 0 {
                                    items.push(MenuItem {
                                        text: "Close All to the Left".into(),
                                        info: None,
                                        command: Some("tab:close-left".into()),
                                        separator: false,
                                    });
                                }
                                if total > 1 {
                                    items.push(MenuItem {
                                        text: "Close All".into(),
                                        info: None,
                                        command: Some("tab:close-all".into()),
                                        separator: false,
                                    });
                                }
                                // Copy-path entries only make sense for an
                                // on-disk file (the doc has a path). Untitled
                                // buffers fall through with just the close
                                // group. The leading item with `separator:
                                // true` is a divider row, not a label; the
                                // real entries follow.
                                if docs.get(i).is_some_and(|d| !d.path.is_empty()) {
                                    items.push(MenuItem {
                                        text: String::new(),
                                        info: None,
                                        command: None,
                                        separator: true,
                                    });
                                    items.push(MenuItem {
                                        text: "Copy Path".into(),
                                        info: None,
                                        command: Some("tab:copy-path".into()),
                                        separator: false,
                                    });
                                    items.push(MenuItem {
                                        text: "Copy Relative Path".into(),
                                        info: None,
                                        command: Some("tab:copy-relative-path".into()),
                                        separator: false,
                                    });
                                }
                                // Estimate the menu size and clamp its origin so
                                // it never renders off-screen. The context menu's
                                // draw_native sizes itself to the widest label.
                                let item_h = style.font_height + style.padding_y;
                                let menu_h = item_h * items.len() as f64 + style.padding_y;
                                let mut menu_w = 0.0_f64;
                                for it in &items {
                                    menu_w = menu_w.max(
                                        draw_ctx.font_width(style.font, &it.text)
                                            + style.padding_x * 2.0,
                                    );
                                }
                                let menu_x = x.min(win_w_tr - menu_w - 2.0).max(0.0);
                                let menu_y = y.min(win_h_tr - menu_h - 2.0).max(tab_h_rc);
                                context_menu.show(menu_x, menu_y, items);
                            }
                            redraw = true;
                            continue;
                        }
                        // Right-click on a sidebar entry: show a rename menu
                        // for that entry rather than the editor context menu.
                        let sidebar_w_rc = if subsystems.has_sidebar() && sidebar_visible {
                            sidebar_width
                        } else {
                            0.0
                        };
                        if subsystems.has_sidebar() && sidebar_visible && *x < sidebar_w_rc {
                            let entry_h = style.font_height + style.padding_y;
                            let sidebar_toolbar_h_rc = if subsystems.has_toolbar() {
                                style.font_height + style.padding_y * 2.0
                            } else {
                                0.0
                            };
                            let sidebar_dir_header_h = style.font_height + style.padding_y;
                            // Clamp sidebar_scroll so the entry index computation stays correct.
                            let real_max_scroll =
                                (sidebar_content_h - sidebar_sb_h).max(0.0);
                            let clamped_scroll = sidebar_scroll.min(real_max_scroll);
                            let raw_idx = ((*y - sidebar_toolbar_h_rc - sidebar_dir_header_h
                                + clamped_scroll)
                                / entry_h)
                                .floor() as usize;
                            let disp_idx = raw_idx.min(sidebar_entries.len().saturating_sub(1));
                            let click_idx: i64 =
                                if !sidebar_entries.is_empty() {
                                    disp_idx as i64
                                } else {
                                    -1
                                };
                            if click_idx >= 0 && (click_idx as usize) < sidebar_entries.len() {
                                let entry = &sidebar_entries[click_idx as usize];
                                sidebar_menu_target = Some((entry.path.clone(), entry.is_dir));
                                let mut items = vec![MenuItem {
                                    text: "New".into(),
                                    info: None,
                                    command: Some("sidebar:new".into()),
                                    separator: false,
                                }];
                                // Rename / Delete are only offered for files;
                                // directories would need recursive path-fixup
                                // across open tabs.
                                if !entry.is_dir {
                                    items.push(MenuItem {
                                        text: String::new(),
                                        info: None,
                                        command: None,
                                        separator: true,
                                    });
                                    items.push(MenuItem {
                                        text: "Rename".into(),
                                        info: None,
                                        command: Some("sidebar:rename".into()),
                                        separator: false,
                                    });
                                    items.push(MenuItem {
                                        text: "Delete".into(),
                                        info: None,
                                        command: Some("sidebar:delete".into()),
                                        separator: false,
                                    });
                                }
                                items.push(MenuItem {
                                    text: String::new(),
                                    info: None,
                                    command: None,
                                    separator: true,
                                });
                                items.push(MenuItem {
                                    text: "Copy Path".into(),
                                    info: None,
                                    command: Some("sidebar:copy-path".into()),
                                    separator: false,
                                });
                                items.push(MenuItem {
                                    text: "Copy Relative Path".into(),
                                    info: None,
                                    command: Some("sidebar:copy-relative-path".into()),
                                    separator: false,
                                });
                                context_menu.show(*x, *y, items);
                                sidebar_menu_pinned_index = Some(disp_idx);
                                redraw = true;
                                continue;
                            }
                        }
                        let mut items = vec![
                            MenuItem {
                                text: "Undo".into(),
                                info: Some("Ctrl+Z".into()),
                                command: Some("doc:undo".into()),
                                separator: false,
                            },
                            MenuItem {
                                text: "Redo".into(),
                                info: Some("Ctrl+Y".into()),
                                command: Some("doc:redo".into()),
                                separator: false,
                            },
                            MenuItem {
                                text: String::new(),
                                info: None,
                                command: None,
                                separator: true,
                            },
                            MenuItem {
                                text: "Cut".into(),
                                info: Some("Ctrl+X".into()),
                                command: Some("doc:cut".into()),
                                separator: false,
                            },
                            MenuItem {
                                text: "Copy".into(),
                                info: Some("Ctrl+C".into()),
                                command: Some("doc:copy".into()),
                                separator: false,
                            },
                            MenuItem {
                                text: "Paste".into(),
                                info: Some("Ctrl+V".into()),
                                command: Some("doc:paste".into()),
                                separator: false,
                            },
                            MenuItem {
                                text: String::new(),
                                info: None,
                                command: None,
                                separator: true,
                            },
                            MenuItem {
                                text: "Select All".into(),
                                info: Some("Ctrl+A".into()),
                                command: Some("doc:select-all".into()),
                                separator: false,
                            },
                        ];
                        if lsp_state.initialized {
                            items.push(MenuItem {
                                text: String::new(),
                                info: None,
                                command: None,
                                separator: true,
                            });
                            items.push(MenuItem {
                                text: "Go to Definition".into(),
                                info: None,
                                command: Some("lsp:go-to-definition".into()),
                                separator: false,
                            });
                            items.push(MenuItem {
                                text: "Find References".into(),
                                info: None,
                                command: Some("lsp:find-references".into()),
                                separator: false,
                            });
                        }
                        let active_doc_path =
                            docs.get(active_tab).map(|d| d.path.as_str()).unwrap_or("");
                        if subsystems.has_terminal()
                            && crate::editor::test_runner::detect_runner_with_fallback(
                                &project_root,
                                active_doc_path,
                            )
                            .is_some()
                        {
                            items.push(MenuItem {
                                text: String::new(),
                                info: None,
                                command: None,
                                separator: true,
                            });
                            items.push(MenuItem {
                                text: "Run All Tests".into(),
                                info: None,
                                command: Some("test:run-all".into()),
                                separator: false,
                            });
                            items.push(MenuItem {
                                text: "Run All Tests in Current File".into(),
                                info: None,
                                command: Some("test:run-in-current-file".into()),
                                separator: false,
                            });
                        }
                        context_menu.show(*x, *y, items);
                        redraw = true;
                        continue;
                    }

                    let sidebar_w = if sidebar_visible { sidebar_width } else { 0.0 };

                    // Sidebar scrollbar grab (lite-xl style). Must run before
                    // sidebar resize and sidebar click handlers, since the
                    // scrollbar lives inside the sidebar rect on the right.
                    if subsystems.has_sidebar()
                        && sidebar_visible
                        && *button == MouseButton::Left
                        && sidebar_content_h > sidebar_sb_h
                        && sidebar_sb_h > 0.0
                    {
                        let sb_w = style.scrollbar_size;
                        let sb_x = sidebar_w - style.divider_size - sb_w;
                        if *x >= sb_x
                            && *x < sb_x + sb_w
                            && *y >= sidebar_sb_top
                            && *y < sidebar_sb_top + sidebar_sb_h
                        {
                            let ratio = sidebar_sb_h / sidebar_content_h;
                            let min_thumb = style.scrollbar_size * 2.0;
                            let thumb_h = (sidebar_sb_h * ratio).max(min_thumb).min(sidebar_sb_h);
                            let max_scroll = (sidebar_content_h - sidebar_sb_h).max(1.0);
                            let scroll_frac = (sidebar_scroll / max_scroll).clamp(0.0, 1.0);
                            let thumb_y = sidebar_sb_top + scroll_frac * (sidebar_sb_h - thumb_h);
                            if *y >= thumb_y && *y < thumb_y + thumb_h {
                                sidebar_sb_drag_offset = *y - thumb_y;
                            } else {
                                sidebar_sb_drag_offset = thumb_h / 2.0;
                                let new_top = (*y - thumb_h / 2.0)
                                    .clamp(sidebar_sb_top, sidebar_sb_top + sidebar_sb_h - thumb_h);
                                let travel = (sidebar_sb_h - thumb_h).max(1.0);
                                let new_frac = (new_top - sidebar_sb_top) / travel;
                                sidebar_scroll_vel = 0.0;
                                sidebar_scroll = (new_frac * max_scroll).max(0.0);
                            }
                            sidebar_sb_dragging = true;
                            redraw = true;
                            continue;
                        }
                    }

                    // Sidebar resize drag: click near the right edge.
                    if subsystems.has_sidebar()
                        && sidebar_visible
                        && (*x - sidebar_w).abs() < 5.0
                        && *button == MouseButton::Left
                    {
                        sidebar_dragging = true;
                        redraw = true;
                        continue;
                    }

                    // Markdown preview resize drag: click near the editor|preview
                    // divider (the left edge of the preview pane).
                    if *button == MouseButton::Left
                        && docs
                            .get(active_tab)
                            .map(|d| {
                                d.preview.enabled
                                    && d.preview.rect.w > 0.0
                                    && (*x - d.preview.rect.x).abs() < 5.0
                            })
                            .unwrap_or(false)
                    {
                        preview_dragging = true;
                        redraw = true;
                        continue;
                    }

                    // Terminal panel resize drag: click on the terminal divider.
                    if subsystems.has_terminal() && terminal.visible && *button == MouseButton::Left
                    {
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let status_h = style.font_height + style.padding_y * 2.0;
                        let tab_h = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let term_h = terminal_h_override.unwrap_or(
                            (wh as f64 * 0.3)
                                .min(wh as f64 - tab_h - status_h - 50.0)
                                .max(80.0),
                        );
                        let term_y = wh as f64 - term_h - status_h;
                        if (*y - term_y).abs() < 5.0 && *x >= sidebar_w {
                            terminal_divider_dragging = true;
                            redraw = true;
                            continue;
                        }
                    }

                    // When the inline new-file input is active, route left clicks:
                    // clicking into the editor commits the new file; clicking
                    // anywhere in the sidebar cancels it.
                    if sidebar_new_file_dir.is_some() && *button == MouseButton::Left {
                        let snap_w = if subsystems.has_sidebar() && sidebar_visible {
                            sidebar_width
                        } else {
                            0.0
                        };
                        if *x >= snap_w {
                            // Commit: create the file and open it.
                            let name = sidebar_new_file_name.trim().to_string();
                            let dir = sidebar_new_file_dir.take().unwrap_or_default();
                            sidebar_new_file_name.clear();
                            sidebar_new_file_cursor = 0;
                            if !name.is_empty() {
                                let full_path = std::path::Path::new(&dir)
                                    .join(&name)
                                    .to_string_lossy()
                                    .to_string();
                                if std::path::Path::new(&full_path).exists() {
                                    info_message = Some((
                                        format!("File already exists: {name}"),
                                        Instant::now(),
                                    ));
                                } else {
                                    match std::fs::write(&full_path, "") {
                                        Ok(()) => {
                                            if subsystems.has_sidebar() && !project_root.is_empty()
                                            {
                                                let in_memory_expanded: HashSet<String> =
                                                    sidebar_entries
                                                        .iter()
                                                        .filter(|e| e.is_dir && e.expanded)
                                                        .map(|e| e.path.clone())
                                                        .collect();
                                                sidebar_entries = scan_for_sidebar(
                                                                                                        &project_root,
                                                    sidebar_show_hidden,
                                                );
                                                restore_expanded_folders(
                                                    &mut sidebar_entries,
                                                    userdir_path,
                                                    sidebar_show_hidden,
                                                    &project_session_key(&project_root),
                                                );
                                                expand_sidebar_from_set(
                                                    &mut sidebar_entries,
                                                    &in_memory_expanded,
                                                    sidebar_show_hidden,
                                                );
                                            }
                                            if open_file_into(&full_path, &mut docs, use_git()) {
                                                autoreload.watch(&full_path);
                                                active_tab = docs.len() - 1;
                                                remember_recent_file(
                                                    &mut recent_files,
                                                    &full_path,
                                                    userdir_path,
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            info_message = Some((
                                                format!("Create failed: {e}"),
                                                Instant::now(),
                                            ));
                                        }
                                    }
                                }
                            }
                            // Fall through so the click still lands in the editor.
                        } else {
                            // Cancel and swallow the click.
                            sidebar_new_file_dir = None;
                            sidebar_new_file_name.clear();
                            sidebar_new_file_cursor = 0;
                            redraw = true;
                            continue;
                        }
                    }

                    // Sidebar click detection.
                    if subsystems.has_sidebar() && sidebar_visible && *x < sidebar_w {
                        use crate::editor::view::DrawContext as _;
                        let ibf = style.icon_big_font;
                        let sidebar_toolbar_h = if subsystems.has_toolbar() {
                            draw_ctx.font_height(ibf) + style.padding_y * 2.0
                        } else {
                            0.0
                        };

                        // Toolbar button click (only when toolbar is enabled).
                        if subsystems.has_toolbar() && *y < sidebar_toolbar_h {
                            let toolbar_buttons: &[(&str, &str)] = &[
                                ("f", "core:new-doc"),
                                ("D", "core:open-file"),
                                ("S", "doc:save"),
                                ("L", "find-replace:find"),
                                ("B", "core:find-command"),
                                ("P", "core:open-user-settings"),
                            ];
                            let mut bx = style.padding_x;
                            let icon_spacing = style.padding_x;
                            let mut clicked_cmd: Option<&str> = None;
                            for (icon, cmd) in toolbar_buttons {
                                let iw = draw_ctx.font_width(ibf, icon);
                                if *x >= bx && *x < bx + iw {
                                    clicked_cmd = Some(cmd);
                                    break;
                                }
                                bx += iw + icon_spacing;
                            }
                            if let Some(cmd) = clicked_cmd {
                                let cmd = cmd.to_string();
                                {
                                    let cmd: String = cmd;
                                    include!("commands_dispatch.rs");
                                }
                            }
                            redraw = true;
                            continue;
                        }

                        let entry_h = style.font_height + style.padding_y;
                        let sidebar_dir_header_h = style.font_height + style.padding_y;
                        let disp_click_idx =
                            ((*y - sidebar_toolbar_h - sidebar_dir_header_h
                                + sidebar_scroll)
                                / entry_h)
                                .floor() as usize;
                        let click_idx = disp_click_idx.min(sidebar_entries.len().saturating_sub(1));
                        if click_idx < sidebar_entries.len() {
                            let entry = &sidebar_entries[click_idx];
                            if entry.is_dir {
                                let was_expanded = sidebar_entries[click_idx].expanded;
                                let path = sidebar_entries[click_idx].path.clone();
                                let depth = sidebar_entries[click_idx].depth;
                                if was_expanded {
                                    // Collapse: remove children.
                                    sidebar_entries[click_idx].expanded = false;
                                    let remove_start = click_idx + 1;
                                    let mut remove_end = remove_start;
                                    while remove_end < sidebar_entries.len()
                                        && sidebar_entries[remove_end].depth > depth
                                    {
                                        remove_end += 1;
                                    }
                                    sidebar_watcher.unwatch_dir(&path);
                                    for entry in
                                        sidebar_entries.iter().take(remove_end).skip(remove_start)
                                    {
                                        if entry.is_dir && entry.expanded {
                                            sidebar_watcher.unwatch_dir(&entry.path.clone());
                                        }
                                    }
                                    sidebar_entries.drain(remove_start..remove_end);
                                } else {
                                    // Expand: insert children.
                                    sidebar_entries[click_idx].expanded = true;
                                    let children =
                                        scan_directory(&path, depth + 1, sidebar_show_hidden);
                                    let insert_at = click_idx + 1;
                                    for (i, child) in children.into_iter().enumerate() {
                                        sidebar_entries.insert(insert_at + i, child);
                                    }
                                    sidebar_watcher.watch_dir(&path);
                                }
                            } else {
                                // Open file as new tab (if not already open).
                                let entry_path = entry.path.clone();
                                let already = docs.iter().position(|d| d.path == entry_path);
                                if let Some(idx) = already {
                                    active_tab = idx;
                                } else {
                                    // Notes mode is single-note-at-a-time —
                                    // close any other notes before opening
                                    // the new one. Autosave will have
                                    // persisted the outgoing buffer
                                    // already, so just drop the tab.
                                    if false {
                                        for d in &docs {
                                            autoreload.unwatch(&d.path);
                                        }
                                        docs.clear();
                                        active_tab = 0;
                                    }
                                    if open_file_into(&entry_path, &mut docs, use_git()) {
                                        autoreload.watch(&entry_path);
                                        active_tab = docs.len() - 1;
                                        remember_recent_file(
                                            &mut recent_files,
                                            &entry_path,
                                            userdir_path,
                                        );
                                    }
                                }
                                // Ensure the switched-to tab has no pending animation.
                                if let Some(doc) = docs.get_mut(active_tab) {
                                    doc.view.target_scroll_y = doc.view.scroll_y;
                                }
                            }
                        }
                        terminal.focused = false;
                        redraw = true;
                        continue;
                    }

                    // Tab bar click detection.
                    let tab_h = if !docs.is_empty() {
                        style.font_height + style.padding_y * 3.0
                    } else {
                        0.0
                    };

                    // Overflow dropdown handling: if the list is open, clicks inside
                    // the list pick that tab; clicks elsewhere close it. If it's
                    // closed, a click on the dropdown button opens it. Left-click
                    // only — right-click in the tab bar should fall through to the
                    // regular context menu path, not toggle the dropdown.
                    if !docs.is_empty() && !false && *button == MouseButton::Left {
                        use crate::editor::view::DrawContext as _;
                        let (ww_tab, _, _, _) = crate::window::get_window_size();
                        let width = ww_tab as f64;
                        let close_btn_w =
                            draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                        let dropdown_btn_w = (style.font_height + style.padding_x * 2.0).ceil();
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
                                + close_btn_w
                                + style.divider_size;
                        }
                        let tabs_overflow = full_total > avail_full;

                        if tab_dropdown_open && tabs_overflow {
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
                            let (_, wh_tab, _, _) = crate::window::get_window_size();
                            let height = wh_tab as f64;
                            let avail_list_w = (width - sidebar_w - 4.0).max(40.0);
                            list_w = list_w.max(120.0).min(avail_list_w);
                            let mut list_x = width - list_w - 2.0;
                            if list_x < sidebar_w + 2.0 {
                                list_x = sidebar_w + 2.0;
                            }
                            let max_list_h = (height - tab_h - 4.0).max(item_h);
                            let raw_list_h = item_h * docs.len() as f64 + style.padding_y;
                            let list_h = raw_list_h.min(max_list_h);
                            let list_y = tab_h + 1.0;
                            if *x >= list_x
                                && *x < list_x + list_w
                                && *y >= list_y
                                && *y < list_y + list_h
                            {
                                let rel = (*y - list_y - style.padding_y / 2.0) / item_h;
                                let idx = rel.floor().max(0.0) as usize;
                                if idx < docs.len() {
                                    active_tab = idx;
                                    if let Some(doc) = docs.get_mut(idx) {
                                        doc.view.target_scroll_y = doc.view.scroll_y;
                                    }
                                }
                                tab_dropdown_open = false;
                                redraw = true;
                                continue;
                            }
                            // Click outside the list dismisses it; also dismiss on a
                            // click on the dropdown button itself (toggle behavior).
                            tab_dropdown_open = false;
                            let btn_x = width - dropdown_btn_w;
                            if *y < tab_h && *x >= btn_x {
                                redraw = true;
                                continue;
                            }
                        } else if tabs_overflow && *y < tab_h {
                            let btn_x = width - dropdown_btn_w;
                            if *x >= btn_x {
                                tab_dropdown_open = true;
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    if *y < tab_h && !docs.is_empty() {
                        terminal.focused = false;
                        use crate::editor::view::DrawContext as _;
                        let (ww_tab_click, _, _, _) = crate::window::get_window_size();
                        let width = ww_tab_click as f64;
                        let close_btn_w =
                            draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                        let dropdown_btn_w = (style.font_height + style.padding_x * 2.0).ceil();

                        // Recompute overflow decision to match the draw pass, and
                        // truncate labels accordingly.
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
                                + close_btn_w
                                + style.divider_size;
                        }
                        let tabs_overflow = full_total > avail_full;
                        let tabs_right_limit = if tabs_overflow {
                            (width - dropdown_btn_w).max(sidebar_w)
                        } else {
                            width
                        };

                        let mut tx = sidebar_w;
                        let mut clicked_close = false;
                        for (i, doc) in docs.iter().enumerate() {
                            let display_label = if tabs_overflow {
                                let base = truncate_tab_name(&doc.name, 10);
                                if doc_is_modified(doc) {
                                    format!("*{base}")
                                } else {
                                    base
                                }
                            } else if doc_is_modified(doc) {
                                format!("*{}", doc.name)
                            } else {
                                doc.name.clone()
                            };
                            let tw = draw_ctx.font_width(style.font, &display_label)
                                + style.padding_x * 2.0
                                + close_btn_w
                                + style.divider_size;
                            // Clip clickable area to the visible region.
                            let click_right = (tx + tw).min(tabs_right_limit);
                            if *x >= tx && *x < click_right {
                                // Check if click is on the close button area (only
                                // when the close icon is actually on-screen).
                                let close_x = tx + tw - close_btn_w - style.divider_size;
                                if *x >= close_x && close_x + close_btn_w <= tabs_right_limit {
                                    if doc_is_modified(doc) {
                                        nag = Nag::UnsavedChanges {
                                            message: nag_msg_close(&doc.name),
                                            tab_to_close: Some(i),
                                        };
                                    } else {
                                        autoreload.unwatch(&doc.path);
                                        if !doc.path.is_empty() {
                                            closed_tabs.retain(|p| p != &doc.path);
                                            closed_tabs.push(doc.path.clone());
                                            if closed_tabs.len() > 25 {
                                                closed_tabs.remove(0);
                                            }
                                        }
                                        docs.remove(i);
                                        if active_tab >= docs.len() && !docs.is_empty() {
                                            active_tab = docs.len() - 1;
                                        }
                                    }
                                    clicked_close = true;
                                } else {
                                    active_tab = i;
                                    tab_tooltip_suppressed = true;
                                    tab_dragging = Some(i);
                                    if let Some(doc) = docs.get_mut(i) {
                                        doc.view.target_scroll_y = doc.view.scroll_y;
                                    }
                                }
                                break;
                            }
                            tx += tw;
                            if tx >= tabs_right_limit {
                                break;
                            }
                        }
                        let _ = clicked_close;
                        redraw = true;
                        continue;
                    }
                    // Terminal click: focus the terminal panel, handle tab/close clicks.
                    if terminal.visible {
                        terminal.focused = true;
                        let (ww, wh, _, _) = crate::window::get_window_size();
                        let win_w = ww as f64;
                        let win_h = wh as f64;
                        let status_h_click = style.font_height + style.padding_y * 2.0;
                        let terminal_h_click = terminal_h_override
                            .unwrap_or(
                                (win_h * 0.3)
                                    .min(win_h - tab_h - status_h_click - 50.0)
                                    .max(80.0),
                            )
                            .min(win_h - tab_h - status_h_click - 50.0)
                            .max(80.0);
                        let term_y_click = win_h - terminal_h_click - status_h_click;
                        let sidebar_w_click = if subsystems.has_sidebar() && sidebar_visible {
                            sidebar_width
                        } else {
                            0.0
                        };
                        let term_x_click = sidebar_w_click;
                        let term_w_click = win_w - sidebar_w_click;
                        let tab_bar_h_click = if !terminal.terminals.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let tab_bar_y = term_y_click + style.divider_size;

                        // Tab bar click (switch / close).
                        if tab_bar_h_click > 0.0
                            && *y >= tab_bar_y
                            && *y < tab_bar_y + tab_bar_h_click
                            && *x >= term_x_click
                            && *x < term_x_click + term_w_click
                        {
                            use crate::editor::view::DrawContext as _;
                            let close_w =
                                draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                            let mut tx = term_x_click;
                            let mut handled = false;
                            let n = terminal.terminals.len();
                            for i in 0..n {
                                let label_w =
                                    draw_ctx.font_width(style.font, &terminal.terminals[i].title);
                                let tw = label_w + style.padding_x * 2.0 + close_w;
                                let close_x = tx + tw - close_w;
                                if *x >= close_x && *x < close_x + close_w {
                                    // Close this terminal.
                                    terminal.active = i;
                                    terminal.close_active();
                                    crate::window::force_invalidate();
                                    handled = true;
                                    break;
                                }
                                if *x >= tx && *x < tx + tw {
                                    terminal.active = i;
                                    handled = true;
                                    break;
                                }
                                tx += tw + style.divider_size;
                            }
                            if handled {
                                redraw = true;
                                continue;
                            }
                        }

                        if *y >= term_y_click && *y < win_h - status_h_click {
                            // Clicking inside the terminal viewport starts a
                            // text selection (mouse-drag copy). If the click
                            // lands on the scrollbar strip on the right
                            // edge, fall through so the dedicated scrollbar
                            // handler below can grab the thumb.
                            use crate::editor::view::DrawContext as _;
                            let char_h_v = style.line_height();
                            let char_w_v = draw_ctx.font_width(style.code_font, "m");
                            let ty_start =
                                term_y_click + style.divider_size + tab_bar_h_click + 2.0;
                            let visible_h =
                                (term_y_click + terminal_h_click - ty_start - style.padding_y)
                                    .max(0.0);
                            let rows_visible = (visible_h / char_h_v).floor().max(1.0) as usize;
                            let sb_w_v = style.scrollbar_size.max(6.0);
                            let on_scrollbar = *x >= term_x_click + term_w_click - sb_w_v
                                && *x < term_x_click + term_w_click
                                && *y >= ty_start
                                && *y < ty_start + char_h_v * rows_visible as f64;
                            if on_scrollbar {
                                // Do not consume -- let the scrollbar
                                // handler below pick this up.
                            } else {
                                let in_viewport = *y >= ty_start
                                    && *y < ty_start + char_h_v * rows_visible as f64
                                    && *x >= term_x_click
                                    && *x < term_x_click + term_w_click - sb_w_v
                                    && char_w_v > 0.0;
                                if in_viewport && *button == MouseButton::Left {
                                    let col = (((*x - term_x_click - style.padding_x) / char_w_v)
                                        .floor()
                                        as i64)
                                        .max(0)
                                        as usize;
                                    let vis_row = (((*y - ty_start) / char_h_v).floor() as i64)
                                        .max(0)
                                        as usize;
                                    if let Some(inst) = terminal.terminals.get_mut(terminal.active)
                                    {
                                        inst.sel_start = Some((vis_row, col));
                                        inst.sel_end = Some((vis_row, col));
                                        inst.sel_dragging = true;
                                    }
                                }
                                // Middle-click pastes the X11 PRIMARY selection
                                // into the shell, matching Linux terminals.
                                if in_viewport
                                    && *button == MouseButton::Middle
                                    && let Some(text) = crate::window::get_primary_selection_text()
                                    && let Some(inst) = terminal.active_terminal()
                                {
                                    let _ = inst.inner.write(text.as_bytes());
                                    inst.scrollback = 0.0;
                                    inst.scrollback_target = 0.0;
                                }
                                redraw = true;
                                continue;
                            }
                        }
                        let _ = ww;
                    }

                    // Minimap click: scroll to the clicked line.
                    if minimap_visible {
                        let (ww, _, _, _) = crate::window::get_window_size();
                        let win_w = ww as f64;
                        let mm_w = 120.0_f64;
                        let mm_x = win_w - mm_w;
                        if *x >= mm_x {
                            let mlh = 4.0_f64;
                            let mm_y = tab_h;
                            let mm_h = {
                                let (_, wh, _, _) = crate::window::get_window_size();
                                let st_h = style.font_height + style.padding_y * 2.0;
                                let terminal_h_click = if terminal.visible {
                                    (wh as f64 * 0.3)
                                        .min(wh as f64 - tab_h - st_h - 50.0)
                                        .max(80.0)
                                } else {
                                    0.0
                                };
                                wh as f64 - tab_h - terminal_h_click - st_h
                            };
                            if let Some(doc) = docs.get_mut(active_tab) {
                                let dv = &mut doc.view;
                                let total_lines =
                                    buffer::with_buffer(dv.buffer_id.unwrap_or(0), |b| {
                                        Ok(b.lines.len())
                                    })
                                    .unwrap_or(0);
                                if total_lines > 0 {
                                    let doc_line_h = style.line_height();
                                    let visible_lines = (dv.rect().h / doc_line_h).ceil() as usize;
                                    let first_visible =
                                        (dv.scroll_y / doc_line_h).floor() as usize + 1;
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
                                    let relative_y = *y - mm_y;
                                    let clicked_line_offset = (relative_y / mlh).floor() as usize;
                                    let target_line =
                                        (minimap_start + clicked_line_offset).clamp(1, total_lines);
                                    let new_scroll = ((target_line as f64 - 1.0) * doc_line_h
                                        - dv.rect().h / 2.0)
                                        .max(0.0);
                                    dv.scroll_y = new_scroll;
                                    dv.target_scroll_y = new_scroll;
                                }
                            }
                            redraw = true;
                            continue;
                        }
                    }

                    // Markdown preview click routing: if the click is in
                    // the preview pane, check checkbox regions first (they
                    // are small targets), then link regions, then bail out
                    // so the click doesn't fall through to the editor.
                    if let Some(doc) = docs.get_mut(active_tab) {
                        if doc.preview.enabled && *button == MouseButton::Left {
                            let pr = doc.preview.rect;
                            if pr.w > 0.0
                                && *x >= pr.x
                                && *x < pr.x + pr.w
                                && *y >= pr.y
                                && *y < pr.y + pr.h
                            {
                                // Checkbox first.
                                let cb = doc
                                    .preview
                                    .checkbox_regions
                                    .iter()
                                    .find(|r| *x >= r.x1 && *x <= r.x2 && *y >= r.y1 && *y <= r.y2)
                                    .cloned();
                                if let Some(cb) = cb {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let src =
                                            buffer::with_buffer(buf_id, |b| Ok(b.lines.join("")))
                                                .unwrap_or_default();
                                        if let Some((line, col, ch)) =
                                            crate::editor::markdown_preview::toggle_task_at(
                                                &src,
                                                cb.source_start,
                                                cb.checked,
                                            )
                                        {
                                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                buffer::push_undo(b);
                                                if line <= b.lines.len() {
                                                    let l = &mut b.lines[line - 1];
                                                    let byte_pos = char_to_byte(l, col - 1);
                                                    if byte_pos < l.len() {
                                                        l.replace_range(
                                                            byte_pos..byte_pos + 1,
                                                            &ch.to_string(),
                                                        );
                                                        b.change_id += 1;
                                                    }
                                                }
                                                Ok(())
                                            });
                                            // Force reparse next draw so the
                                            // checkbox visibly fills/clears.
                                            doc.preview.cached_change_id = -1;
                                        }
                                    }
                                    redraw = true;
                                    continue;
                                }
                                // Link next.
                                if let Some(lr) =
                                    doc.preview.link_regions.iter().find(|r| {
                                        *x >= r.x1 && *x <= r.x2 && *y >= r.y1 && *y <= r.y2
                                    })
                                {
                                    crate::editor::markdown_preview::open_url(&lr.href);
                                }
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Editor scrollbar mouse-down: grab the thumb (lite-xl
                    // style). If the click is on the thumb itself, we keep
                    // the existing scroll and remember the offset within the
                    // thumb so dragging feels like grabbing a handle. If the
                    // click is on the empty track, we jump so the thumb
                    // centers under the cursor, then grab for the drag.
                    if let Some(doc) = docs.get_mut(active_tab) {
                        let dv_rect = doc.view.rect();
                        let sb_w = style.scrollbar_size;
                        let sb_x = dv_rect.x + dv_rect.w - sb_w;
                        if *x >= sb_x
                            && *x < sb_x + sb_w
                            && *y >= dv_rect.y
                            && *y < dv_rect.y + dv_rect.h
                            && dv_rect.h > 0.0
                        {
                            let line_h_sb = style.line_height();
                            let total_lines = doc
                                .view
                                .buffer_id
                                .and_then(|id| buffer::with_buffer(id, |b| Ok(b.lines.len())).ok())
                                .unwrap_or(1) as f64;
                            let total_h = total_lines * line_h_sb;
                            if total_h > dv_rect.h {
                                let ratio = dv_rect.h / total_h;
                                let min_thumb = style.scrollbar_size * 2.0;
                                let thumb_h = (dv_rect.h * ratio).max(min_thumb).min(dv_rect.h);
                                let scroll_frac =
                                    doc.view.scroll_y / (total_h - dv_rect.h).max(1.0);
                                let thumb_y = dv_rect.y + scroll_frac * (dv_rect.h - thumb_h);
                                if *y >= thumb_y && *y < thumb_y + thumb_h {
                                    editor_sb_drag_offset = *y - thumb_y;
                                } else {
                                    editor_sb_drag_offset = thumb_h / 2.0;
                                    let new_top = (*y - thumb_h / 2.0)
                                        .clamp(dv_rect.y, dv_rect.y + dv_rect.h - thumb_h);
                                    let new_frac = (new_top - dv_rect.y) / (dv_rect.h - thumb_h);
                                    let new_scroll = (new_frac * (total_h - dv_rect.h)).max(0.0);
                                    doc.view.target_scroll_y = new_scroll;
                                    doc.view.scroll_y = new_scroll;
                                    editor_scroll_vel = 0.0;
                                }
                                editor_sb_dragging = true;
                                redraw = true;
                                continue;
                            }
                        }
                    }

                    // Terminal scrollbar click: set scrollback_target by the
                    // clicked fraction of the track.
                    if subsystems.has_terminal() && terminal.visible {
                        let (ww, wh, _, _) = crate::window::get_window_size();
                        let win_w = ww as f64;
                        let win_h = wh as f64;
                        let status_h_sc = style.font_height + style.padding_y * 2.0;
                        let tab_h_sc = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let terminal_h_sc = terminal_h_override
                            .unwrap_or(
                                (win_h * 0.3)
                                    .min(win_h - tab_h_sc - status_h_sc - 50.0)
                                    .max(80.0),
                            )
                            .min(win_h - tab_h_sc - status_h_sc - 50.0)
                            .max(80.0);
                        let term_y_sc = win_h - terminal_h_sc - status_h_sc;
                        let sidebar_w_sc = if subsystems.has_sidebar() && sidebar_visible {
                            sidebar_width
                        } else {
                            0.0
                        };
                        let term_x_sc = sidebar_w_sc;
                        let term_w_sc = win_w - sidebar_w_sc;
                        let tab_bar_h_sc = if !terminal.terminals.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let char_h_sc = style.line_height();
                        let ty_start = term_y_sc + style.divider_size + tab_bar_h_sc + 2.0;
                        let visible_h =
                            (term_y_sc + terminal_h_sc - ty_start - style.padding_y).max(0.0);
                        let rows_visible = (visible_h / char_h_sc).floor().max(1.0) as usize;
                        let sb_w_sc = style.scrollbar_size.max(6.0);
                        let sb_x_sc = term_x_sc + term_w_sc - sb_w_sc;
                        let sb_h_sc = char_h_sc * rows_visible as f64;
                        if *x >= sb_x_sc
                            && *x < sb_x_sc + sb_w_sc
                            && *y >= ty_start
                            && *y < ty_start + sb_h_sc
                        {
                            if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                                let cap = inst.tbuf.history_len() as f64;
                                if cap > 0.0 && sb_h_sc > 0.0 {
                                    let total = cap + rows_visible as f64;
                                    let ratio = (rows_visible as f64 / total).clamp(0.0, 1.0);
                                    let min_thumb = sb_w_sc * 2.0;
                                    let thumb_h = (sb_h_sc * ratio).max(min_thumb).min(sb_h_sc);
                                    let pos_from_top = (cap - inst.scrollback_target) / cap;
                                    let thumb_y = ty_start + pos_from_top * (sb_h_sc - thumb_h);
                                    if *y >= thumb_y && *y < thumb_y + thumb_h {
                                        terminal_sb_drag_offset = *y - thumb_y;
                                    } else {
                                        terminal_sb_drag_offset = thumb_h / 2.0;
                                        let new_top = (*y - thumb_h / 2.0)
                                            .clamp(ty_start, ty_start + sb_h_sc - thumb_h);
                                        let travel = (sb_h_sc - thumb_h).max(1.0);
                                        let new_from_top = (new_top - ty_start) / travel;
                                        inst.scrollback_target = (1.0 - new_from_top) * cap;
                                    }
                                    terminal_sb_dragging = true;
                                    redraw = true;
                                    continue;
                                }
                            }
                        }
                    }

                    // Test-runner badge hit-test: if the click lands on
                    // one of the inline "Run test" hints, dispatch a
                    // single-test run and skip caret placement.
                    if !test_badges.is_empty() {
                        let hit = test_badges
                            .iter()
                            .find(|r| *x >= r.x1 && *x < r.x2 && *y >= r.y1 && *y < r.y2);
                        if let Some(region) = hit {
                            if let Some(test) = active_tests.get(region.test_index) {
                                let doc_path = docs
                                    .get(active_tab)
                                    .map(|d| d.path.clone())
                                    .unwrap_or_default();
                                if !doc_path.is_empty() {
                                    pending_single_test = Some((doc_path, test.name.clone()));
                                    {
                                        let cmd: String = "test:run-single".to_string();
                                        include!("commands_dispatch.rs");
                                    }
                                }
                            }
                            redraw = true;
                            continue;
                        }
                    }

                    // Middle-click pastes the X11 PRIMARY selection at the
                    // click point, the standard Linux convention. Only acts
                    // inside the editor viewport rect; consumes the event so it
                    // never falls through to cursor placement / drag-select.
                    if *button == MouseButton::Middle {
                        if let Some(text) = crate::window::get_primary_selection_text()
                            && let Some(doc) = docs.get(active_tab)
                        {
                            let dv = &doc.view;
                            if let Some(buf_id) = dv.buffer_id {
                                let dvr = dv.rect();
                                let in_editor = *x >= dvr.x
                                    && *x < dvr.x + dvr.w
                                    && *y >= dvr.y
                                    && *y < dvr.y + dvr.h;
                                if in_editor {
                                    let line_h = style.line_height();
                                    let gutter_w = dv.gutter_width;
                                    let text_x_start =
                                        dv.rect().x + gutter_w + style.padding_x - dv.scroll_x;
                                    let (click_line, click_col) = click_to_doc_pos(
                                        dv,
                                        buf_id,
                                        &doc.cached_render,
                                        *x,
                                        *y,
                                        text_x_start,
                                        line_h,
                                        &style,
                                        &mut draw_ctx,
                                    );
                                    let text = if config.format_on_paste {
                                        convert_paste_indent(
                                            &text,
                                            &doc.indent_type,
                                            doc.indent_size,
                                        )
                                    } else {
                                        text
                                    };
                                    let _ = buffer::with_buffer_mut(buf_id, |b| {
                                        let line = click_line.min(b.lines.len()).max(1);
                                        let max_col =
                                            char_count(b.lines[line - 1].trim_end_matches('\n'))
                                                + 1;
                                        let col = click_col.min(max_col);
                                        b.selections = vec![line, col, line, col];
                                        insert_text_at_caret(b, &text);
                                        Ok(())
                                    });
                                }
                            }
                        }
                        redraw = true;
                        continue;
                    }

                    // Completion popup: click item to accept, click outside to dismiss.
                    if *button == MouseButton::Left && completion.visible {
                        let (px, py, pw, ph) = completion.rect;
                        if *x >= px && *x < px + pw && *y >= py && *y < py + ph {
                            let item_h = style.font_height + style.padding_y;
                            let row = ((*y - py - style.padding_y / 2.0) / item_h) as usize;
                            let idx = completion.scroll_offset + row;
                            if idx < completion.items.len() {
                                completion.selected = idx;
                                if let Some((_, _, insert_text)) =
                                    completion.items.get(completion.selected)
                                {
                                    let text = insert_text.clone();
                                    if let Some(doc) = docs.get_mut(active_tab) {
                                        if let Some(buf_id) = doc.view.buffer_id {
                                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                buffer::push_undo(b);
                                                let line = *b.selections.first().unwrap_or(&1);
                                                let col = *b.selections.get(1).unwrap_or(&1);
                                                if line <= b.lines.len() {
                                                    let l = &b.lines[line - 1];
                                                    let chars: Vec<char> = l.chars().collect();
                                                    let col_idx = (col - 1).min(chars.len());
                                                    let mut word_start = col_idx;
                                                    while word_start > 0 {
                                                        let c = chars[word_start - 1];
                                                        if c.is_alphanumeric() || c == '_' {
                                                            word_start -= 1;
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    let l = &mut b.lines[line - 1];
                                                    let byte_start = char_to_byte(l, word_start);
                                                    let byte_end = char_to_byte(l, col - 1);
                                                    l.replace_range(byte_start..byte_end, &text);
                                                    let new_col =
                                                        word_start + 1 + text.chars().count();
                                                    b.selections[0] = line;
                                                    b.selections[1] = new_col;
                                                    b.selections[2] = line;
                                                    b.selections[3] = new_col;
                                                }
                                                Ok(())
                                            });
                                        }
                                    }
                                }
                                completion.hide();
                            }
                        } else {
                            completion.hide();
                        }
                        redraw = true;
                        continue;
                    }

                    // CmdView: click suggestion to select it; click outside to dismiss.
                    if *button == MouseButton::Left && cmdview_active {
                        let (ww_cv, _, _, _) = crate::window::get_window_size();
                        let width_cv = ww_cv as f64;
                        let cv_w = (width_cv * 0.7).max(500.0).min(width_cv - 20.0);
                        let cv_x = (width_cv - cv_w) / 2.0;
                        let line_h = style.font_height + style.padding_y;
                        let max_visible = 15usize;
                        let visible_count = cmdview_suggestions.len().min(max_visible);
                        let cv_h = line_h * (visible_count as f64 + 1.0) + style.padding_y * 2.0;
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
                        if *x >= cv_x && *x < cv_x + cv_w && *y >= cv_y && *y < cv_y + cv_h {
                            let input_y = cv_y + style.padding_y;
                            let suggestion_start = input_y + line_h + style.divider_size;
                            if *y >= suggestion_start {
                                let row = ((*y - suggestion_start) / line_h) as usize;
                                if row < cmdview_suggestions.len() {
                                    cmdview_selected = row;
                                }
                            }
                        } else {
                            cmdview_active = false;
                        }
                        redraw = true;
                        continue;
                    }

                    // Command palette: click command to activate, click outside to dismiss.
                    if *button == MouseButton::Left && palette_active {
                        let (ww_pal, _, _, _) = crate::window::get_window_size();
                        let width_pal = ww_pal as f64;
                        let pal_w = (width_pal * 0.5).max(400.0).min(width_pal - 20.0);
                        let pal_x = (width_pal - pal_w) / 2.0;
                        let pal_y = style.padding_y * 2.0;
                        let line_h = style.font_height + style.padding_y;
                        let max_visible = 12usize;
                        let visible = palette_results.len().min(max_visible);
                        let pal_h = line_h * (visible as f64 + 1.0) + style.padding_y * 2.0;
                        if *x >= pal_x && *x < pal_x + pal_w && *y >= pal_y && *y < pal_y + pal_h {
                            let input_y = pal_y + style.padding_y;
                            let suggestion_start = input_y + line_h + style.divider_size;
                            if *y >= suggestion_start {
                                let row = ((*y - suggestion_start) / line_h) as usize;
                                if row < palette_results.len() {
                                    palette_selected = row;
                                    let (cmd, _) = &palette_results[palette_selected];
                                    let cmd = cmd.clone();
                                    palette_active = false;
                                    include!("commands_dispatch.rs");
                                }
                            }
                        } else {
                            palette_active = false;
                        }
                        redraw = true;
                        continue;
                    }

                    // Theme picker: click theme to select, click outside to dismiss.
                    if *button == MouseButton::Left && theme_picker_active {
                        let (ww_pal, _, _, _) = crate::window::get_window_size();
                        let width_pal = ww_pal as f64;
                        let pal_w = (width_pal * 0.4).max(300.0).min(width_pal - 20.0);
                        let pal_x = (width_pal - pal_w) / 2.0;
                        let pal_y = style.padding_y * 2.0;
                        let line_h = style.font_height + style.padding_y;
                        let max_visible = 12usize;
                        let visible = theme_picker_results.len().min(max_visible);
                        let pal_h = line_h * (visible as f64 + 1.0) + style.padding_y * 2.0;
                        if *x >= pal_x && *x < pal_x + pal_w && *y >= pal_y && *y < pal_y + pal_h {
                            let input_y = pal_y + style.padding_y;
                            let suggestion_start = input_y + line_h + style.divider_size;
                            if *y >= suggestion_start {
                                let row = ((*y - suggestion_start) / line_h) as usize;
                                if row < theme_picker_results.len() {
                                    theme_picker_selected = row;
                                    // Confirm selected theme.
                                    if let Some((name, _)) = theme_picker_results.get(theme_picker_selected) {
                                        current_theme_idx = available_themes.iter().position(|t| t == name).unwrap_or(0);
                                        // Apply theme & invalidate cache so syntax colours refresh.
                                        let tp = Path::new(datadir)
                                            .join("assets")
                                            .join("themes")
                                            .join(format!("{name}.json"))
                                            .to_string_lossy()
                                            .into_owned();
                                        if let Ok(palette) = crate::editor::style::load_theme_palette(&tp) {
                                            apply_theme_to_style(&mut style, &palette);
                                            crate::editor::style_ctx::set_current_style(style.clone());
                                            // Invalidate all render caches so syntax colours refresh.
                                            pending_render_cache = None;
                                            for doc in &mut docs {
                                                doc.cached_change_id = -1;
                                            }
                                        }
                                        // Persist to config.toml
                                        let config_path = std::path::Path::new(userdir).join("config.toml");
                                        let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
                                        if let Ok(mut doc) = existing.parse::<toml::Value>() {
                                            if let toml::Value::Table(ref mut map) = doc {
                                                map.insert("theme".to_string(), toml::Value::String(name.clone()));
                                            }
                                            if let Ok(out) = toml::to_string(&doc) {
                                                let _ = std::fs::write(&config_path, out);
                                            }
                                        }
                                    }
                                    theme_picker_active = false;
                                    theme_picker_original_style = None;
                                }
                            }
                        } else {
                            // Restore original theme on dismiss.
                            if let Some(orig) = theme_picker_original_style.take() {
                                style = orig;
                                current_theme_idx = theme_picker_original_idx;
                            }
                            theme_picker_active = false;
                        }
                        redraw = true;
                        continue;
                    }

                    if let Some(doc) = docs.get_mut(active_tab) {
                        terminal.focused = false;
                        let dv = &mut doc.view;
                        if let Some(buf_id) = dv.buffer_id {
                            // When the editor is split-paned with a preview,
                            // reject clicks that land outside its rect so
                            // cursor/selection math isn't fed stray coords.
                            let dvr = dv.rect();
                            if *x < dvr.x || *x >= dvr.x + dvr.w {
                                redraw = true;
                                continue;
                            }
                            let line_h = style.line_height();
                            let gutter_w = dv.gutter_width;
                            let text_x_start =
                                dv.rect().x + gutter_w + style.padding_x - dv.scroll_x;
                            let (click_line, click_col) = click_to_doc_pos(
                                dv,
                                buf_id,
                                &doc.cached_render,
                                *x,
                                *y,
                                text_x_start,
                                line_h,
                                &style,
                                &mut draw_ctx,
                            );
                            let extending = shift_held || modifiers.shift;
                            let n_clicks = *clicks;
                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                let line = click_line.min(b.lines.len()).max(1);
                                let max_col =
                                    char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                                let col = click_col.min(max_col);
                                if n_clicks >= 3 && !extending {
                                    // Triple-click selects the whole clicked
                                    // line, matching Lite-XL's
                                    // `doc:set-cursor-line` binding.
                                    b.selections = vec![line, 1, line, max_col];
                                } else if n_clicks == 2 && !extending {
                                    // Double-click selects the word under the
                                    // cursor. Word chars are alphanumeric or
                                    // '_', matching the existing
                                    // word-movement commands.
                                    let text = b.lines[line - 1].trim_end_matches('\n');
                                    let chars: Vec<char> = text.chars().collect();
                                    let is_word = |c: char| c.is_alphanumeric() || c == '_';
                                    let idx = (col - 1).min(chars.len());
                                    if idx < chars.len() && is_word(chars[idx]) {
                                        let mut start = idx;
                                        while start > 0 && is_word(chars[start - 1]) {
                                            start -= 1;
                                        }
                                        let mut end = idx;
                                        while end < chars.len() && is_word(chars[end]) {
                                            end += 1;
                                        }
                                        b.selections = vec![line, start + 1, line, end + 1];
                                    } else if idx > 0 && is_word(chars[idx - 1]) {
                                        // Click landed just past the end of a
                                        // word (e.g. on trailing whitespace);
                                        // still select that word.
                                        let mut start = idx - 1;
                                        while start > 0 && is_word(chars[start - 1]) {
                                            start -= 1;
                                        }
                                        b.selections = vec![line, start + 1, line, idx + 1];
                                    } else {
                                        b.selections = vec![line, col, line, col];
                                    }
                                } else if extending && b.selections.len() >= 4 {
                                    // Shift+click extends the existing selection: keep the
                                    // anchor (selections[0..2]) and only move the cursor end.
                                    b.selections.truncate(4);
                                    b.selections[2] = line;
                                    b.selections[3] = col;
                                } else {
                                    b.selections = vec![line, col, line, col];
                                }
                                Ok(())
                            });
                            editor_mouse_down = true;
                        }
                    }
                    redraw = true;
                }
                EditorEvent::MouseMoved { x, y, .. } => {
                    mouse_x = *x;
                    mouse_y = *y;

                    // Hover highlight for the context menu (right-click on a
                    // tab, sidebar entry, doc area, or the tab-overflow
                    // dropdown). Without this `selected` only changes via
                    // keyboard up/down, so a freshly-opened menu had no
                    // active-row indicator.
                    if context_menu.visible {
                        // Use the actual flipped draw position so hover matches
                        // the on-screen rect (auto-flipped when near edges).
                        let (menu_x, menu_y, menu_w, menu_h) = context_menu.render_rect;
                        let item_h = style.font_height + style.padding_y;
                        if menu_h > 0.0
                            && *x >= menu_x
                            && *x <= menu_x + menu_w
                            && *y >= menu_y
                            && *y <= menu_y + menu_h
                        {
                            let rel = (*y - menu_y - style.padding_y / 2.0) / item_h;
                            let idx = rel.floor().max(0.0) as usize;
                            if idx < context_menu.items.len()
                                && !context_menu.items[idx].separator
                            {
                                context_menu.selected = Some(idx);
                            } else {
                                context_menu.selected = None;
                            }
                        } else {
                            context_menu.selected = None;
                        }
                        redraw = true;
                    }
                    // Tab drag reorder.
                    if let Some(drag_idx) = tab_dragging {
                        let tab_h = style.font_height + style.padding_y * 3.0;
                        if *y < tab_h {
                            use crate::editor::view::DrawContext as _;
                            let sidebar_w = if sidebar_visible { sidebar_width } else { 0.0 };
                            let close_w =
                                draw_ctx.font_width(style.icon_font, "C") + style.padding_x;
                            // Match the draw pass: if the tab bar overflows, labels
                            // are truncated, so the drag hit-test must use the same
                            // widths or reorder lands on the wrong tab.
                            let (ww_dr, _, _, _) = crate::window::get_window_size();
                            let width = ww_dr as f64;
                            let dropdown_btn_w = (style.font_height + style.padding_x * 2.0).ceil();
                            let avail_full = (width - sidebar_w).max(0.0);
                            let mut full_total = 0.0_f64;
                            for doc in docs.iter() {
                                let l = if doc_is_modified(doc) {
                                    format!("*{}", doc.name)
                                } else {
                                    doc.name.clone()
                                };
                                full_total += draw_ctx.font_width(style.font, &l)
                                    + style.padding_x * 2.0
                                    + close_w
                                    + style.divider_size;
                            }
                            let tabs_overflow = full_total > avail_full;
                            let tabs_right_limit = if tabs_overflow {
                                (width - dropdown_btn_w).max(sidebar_w)
                            } else {
                                width
                            };
                            let mut tx = sidebar_w;
                            for (i, doc) in docs.iter().enumerate() {
                                let label = if tabs_overflow {
                                    let base = truncate_tab_name(&doc.name, 10);
                                    if doc_is_modified(doc) {
                                        format!("*{base}")
                                    } else {
                                        base
                                    }
                                } else if doc_is_modified(doc) {
                                    format!("*{}", doc.name)
                                } else {
                                    doc.name.clone()
                                };
                                let tw = draw_ctx.font_width(style.font, &label)
                                    + style.padding_x * 2.0
                                    + close_w
                                    + style.divider_size;
                                let hit_right = (tx + tw).min(tabs_right_limit);
                                if *x >= tx && *x < hit_right && i != drag_idx {
                                    docs.swap(i, drag_idx);
                                    tab_dragging = Some(i);
                                    active_tab = i;
                                    redraw = true;
                                    break;
                                }
                                tx += tw;
                                if tx >= tabs_right_limit {
                                    break;
                                }
                            }
                        }
                        continue;
                    }
                    // Editor scrollbar drag: move the thumb so its grabbed
                    // point stays under the cursor, then derive scroll.
                    if editor_sb_dragging {
                        if let Some(doc) = docs.get_mut(active_tab) {
                            let dv_rect = doc.view.rect();
                            let line_h_sb = style.line_height();
                            let total_lines = doc
                                .view
                                .buffer_id
                                .and_then(|id| buffer::with_buffer(id, |b| Ok(b.lines.len())).ok())
                                .unwrap_or(1) as f64;
                            let total_h = total_lines * line_h_sb;
                            if total_h > dv_rect.h && dv_rect.h > 0.0 {
                                let ratio = dv_rect.h / total_h;
                                let min_thumb = style.scrollbar_size * 2.0;
                                let thumb_h = (dv_rect.h * ratio).max(min_thumb).min(dv_rect.h);
                                let new_top = (*y - editor_sb_drag_offset)
                                    .clamp(dv_rect.y, dv_rect.y + dv_rect.h - thumb_h);
                                let travel = (dv_rect.h - thumb_h).max(1.0);
                                let new_frac = (new_top - dv_rect.y) / travel;
                                let new_scroll = (new_frac * (total_h - dv_rect.h)).max(0.0);
                                doc.view.target_scroll_y = new_scroll;
                                doc.view.scroll_y = new_scroll;
                                editor_scroll_vel = 0.0;
                                redraw = true;
                            }
                        }
                        continue;
                    }

                    // Terminal scrollbar drag: recompute scrollback from
                    // mouse y. Must come before the selection drag, so a
                    // mouse-down on the track doesn't turn into a cell
                    // selection on drag.
                    if terminal_sb_dragging && subsystems.has_terminal() && terminal.visible {
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let win_h = wh as f64;
                        let status_h_sm = style.font_height + style.padding_y * 2.0;
                        let tab_h_sm = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let terminal_h_sm = terminal_h_override
                            .unwrap_or(
                                (win_h * 0.3)
                                    .min(win_h - tab_h_sm - status_h_sm - 50.0)
                                    .max(80.0),
                            )
                            .min(win_h - tab_h_sm - status_h_sm - 50.0)
                            .max(80.0);
                        let term_y_sm = win_h - terminal_h_sm - status_h_sm;
                        let tab_bar_h_sm = if !terminal.terminals.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let char_h_sm = style.line_height();
                        let ty_start = term_y_sm + style.divider_size + tab_bar_h_sm + 2.0;
                        let visible_h =
                            (term_y_sm + terminal_h_sm - ty_start - style.padding_y).max(0.0);
                        let rows_visible = (visible_h / char_h_sm).floor().max(1.0) as usize;
                        let sb_h = char_h_sm * rows_visible as f64;
                        let sb_w_sm = style.scrollbar_size.max(6.0);
                        if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                            let cap = inst.tbuf.history_len() as f64;
                            if cap > 0.0 && sb_h > 0.0 {
                                let total = cap + rows_visible as f64;
                                let ratio = (rows_visible as f64 / total).clamp(0.0, 1.0);
                                let min_thumb = sb_w_sm * 2.0;
                                let thumb_h = (sb_h * ratio).max(min_thumb).min(sb_h);
                                let new_top = (*y - terminal_sb_drag_offset)
                                    .clamp(ty_start, ty_start + sb_h - thumb_h);
                                let travel = (sb_h - thumb_h).max(1.0);
                                let new_from_top = (new_top - ty_start) / travel;
                                inst.scrollback_target = (1.0 - new_from_top) * cap;
                                redraw = true;
                            }
                        }
                        continue;
                    }

                    // Terminal: extend the active selection while drag is in
                    // progress. Done before any other mouse-move branch
                    // because the terminal sits at the bottom of the
                    // window and its drag shouldn't trigger sidebar resize
                    // or editor caret drag.
                    if subsystems.has_terminal() && terminal.visible {
                        use crate::editor::view::DrawContext as _;
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let win_h = wh as f64;
                        let status_h_m = style.font_height + style.padding_y * 2.0;
                        let tab_h_m = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let terminal_h_m = terminal_h_override
                            .unwrap_or(
                                (win_h * 0.3)
                                    .min(win_h - tab_h_m - status_h_m - 50.0)
                                    .max(80.0),
                            )
                            .min(win_h - tab_h_m - status_h_m - 50.0)
                            .max(80.0);
                        let term_y_m = win_h - terminal_h_m - status_h_m;
                        let sidebar_w_m = if subsystems.has_sidebar() && sidebar_visible {
                            sidebar_width
                        } else {
                            0.0
                        };
                        let term_x_m = sidebar_w_m;
                        let tab_bar_h_m = if !terminal.terminals.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let char_h_m = style.line_height();
                        let char_w_m = draw_ctx.font_width(style.code_font, "m");
                        let ty_start = term_y_m + style.divider_size + tab_bar_h_m + 2.0;
                        let visible_h =
                            (term_y_m + terminal_h_m - ty_start - style.padding_y).max(0.0);
                        let rows_visible = (visible_h / char_h_m).floor().max(1.0) as usize;
                        if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                            if inst.sel_dragging && char_w_m > 0.0 {
                                let col = ((*x - term_x_m - style.padding_x) / char_w_m)
                                    .floor()
                                    .max(0.0) as usize;
                                let vis_row = (((*y - ty_start) / char_h_m).floor().max(0.0)
                                    as usize)
                                    .min(rows_visible.saturating_sub(1));
                                inst.sel_end = Some((vis_row, col));
                                redraw = true;
                            }
                        }
                    }
                    if sidebar_sb_dragging {
                        if sidebar_content_h > sidebar_sb_h && sidebar_sb_h > 0.0 {
                            let ratio = sidebar_sb_h / sidebar_content_h;
                            let min_thumb = style.scrollbar_size * 2.0;
                            let thumb_h = (sidebar_sb_h * ratio).max(min_thumb).min(sidebar_sb_h);
                            let new_top = (*y - sidebar_sb_drag_offset)
                                .clamp(sidebar_sb_top, sidebar_sb_top + sidebar_sb_h - thumb_h);
                            let travel = (sidebar_sb_h - thumb_h).max(1.0);
                            let new_frac = (new_top - sidebar_sb_top) / travel;
                            let max_scroll = (sidebar_content_h - sidebar_sb_h).max(1.0);
                            sidebar_scroll_vel = 0.0;
                            sidebar_scroll = (new_frac * max_scroll).max(0.0);
                            redraw = true;
                        }
                        continue;
                    }
                    if sidebar_dragging {
                        let (ww, _, _, _) = crate::window::get_window_size();
                        let max_sidebar = (ww as f64 * 0.9).max(MIN_SIDEBAR_W);
                        sidebar_width = x.clamp(MIN_SIDEBAR_W, max_sidebar);
                        redraw = true;
                    } else if terminal_divider_dragging {
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let status_h = style.font_height + style.padding_y * 2.0;
                        let new_h = (wh as f64 - y - status_h).max(80.0).min(wh as f64 * 0.8);
                        terminal_h_override = Some(new_h);
                        redraw = true;
                    } else if preview_dragging {
                        // Recover the shared content area from the editor (left)
                        // and preview (right) rects so the split is expressed as
                        // a window-relative fraction that survives resizes.
                        if let Some(doc) = docs.get(active_tab) {
                            let content_x = doc.view.rect().x;
                            let content_right = doc.preview.rect.x + doc.preview.rect.w;
                            let content_w = (content_right - content_x).max(1.0);
                            preview_split = ((*x - content_x) / content_w)
                                .clamp(MIN_PREVIEW_SPLIT, MAX_PREVIEW_SPLIT);
                        }
                        redraw = true;
                    } else if editor_mouse_down {
                        // Drag selection: update cursor position while keeping anchor.
                    if let Some(doc) = docs.get_mut(active_tab) {
                        terminal.focused = false;
                        let dv = &mut doc.view;
                        if let Some(buf_id) = dv.buffer_id {
                                let line_h = style.line_height();
                                let gutter_w = dv.gutter_width;
                                let text_x_start =
                                    dv.rect().x + gutter_w + style.padding_x - dv.scroll_x;
                                let (drag_line, drag_col) = click_to_doc_pos(
                                    dv,
                                    buf_id,
                                    &doc.cached_render,
                                    *x,
                                    *y,
                                    text_x_start,
                                    line_h,
                                    &style,
                                    &mut draw_ctx,
                                );
                                let _ = buffer::with_buffer_mut(buf_id, |b| {
                                    let line = drag_line.min(b.lines.len()).max(1);
                                    let max_col =
                                        char_count(b.lines[line - 1].trim_end_matches('\n')) + 1;
                                    b.selections[2] = line;
                                    b.selections[3] = drag_col.min(max_col);
                                    Ok(())
                                });
                                redraw = true;
                            }
                        }
                    }
                    let sidebar_w = if sidebar_visible { sidebar_width } else { 0.0 };
                    // Hand cursor when hovering a markdown preview link.
                    let hover_link =
                        docs.get(active_tab)
                            .map(|d| {
                                d.preview.enabled
                                    && d.preview.link_regions.iter().any(|r| {
                                        *x >= r.x1 && *x <= r.x2 && *y >= r.y1 && *y <= r.y2
                                    })
                            })
                            .unwrap_or(false);
                    let hover_preview_divider = docs
                        .get(active_tab)
                        .map(|d| {
                            d.preview.enabled
                                && d.preview.rect.w > 0.0
                                && (*x - d.preview.rect.x).abs() < 5.0
                        })
                        .unwrap_or(false);
                    let hover_terminal_divider = if subsystems.has_terminal() && terminal.visible {
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let status_h = style.font_height + style.padding_y * 2.0;
                        let tab_h = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let term_h = terminal_h_override.unwrap_or(
                            (wh as f64 * 0.3)
                                .min(wh as f64 - tab_h - status_h - 50.0)
                                .max(80.0),
                        );
                        let term_y = wh as f64 - term_h - status_h;
                        (*y - term_y).abs() < 5.0 && *x >= sidebar_w
                    } else {
                        false
                    };
                    if hover_link {
                        crate::window::set_cursor("hand");
                    } else if (subsystems.has_sidebar()
                        && sidebar_visible
                        && (*x - sidebar_w).abs() < 5.0)
                        || hover_preview_divider
                        || preview_dragging
                        || terminal_divider_dragging
                        || hover_terminal_divider
                    {
                        crate::window::set_cursor("sizev");
                    } else if !sidebar_dragging
                        && !editor_mouse_down
                        && !preview_dragging
                        && !terminal_divider_dragging
                    {
                        crate::window::set_cursor("arrow");
                    } else if editor_mouse_down {
                        crate::window::set_cursor("ibeam");
                    }

                    // Hover tooltip tracking: map the cursor to a
                    // (line, col) over the active doc. If a diagnostic
                    // is under the cursor, surface its message
                    // immediately. Otherwise note the position + time
                    // so the debounce loop below can fire a deferred
                    // `textDocument/hover` request.
                    let new_doc_pos: Option<(usize, usize)> = (|| {
                        if editor_mouse_down || sidebar_dragging {
                            return None;
                        }
                        let doc = docs.get(active_tab)?;
                        let buf_id = doc.view.buffer_id?;
                        let dv = &doc.view;
                        let dvr = dv.rect();
                        if *x < dvr.x || *x >= dvr.x + dvr.w || *y < dvr.y || *y >= dvr.y + dvr.h {
                            return None;
                        }
                        let line_h = style.line_height();
                        let gutter_w = dv.gutter_width;
                        let text_x_start = dv.rect().x + gutter_w + style.padding_x - dv.scroll_x;
                        if *x < text_x_start - style.padding_x {
                            return None;
                        }
                        let (line, col) = click_to_doc_pos(
                            dv,
                            buf_id,
                            &doc.cached_render,
                            *x,
                            *y,
                            text_x_start,
                            line_h,
                            &style,
                            &mut draw_ctx,
                        );
                        Some((line, col))
                    })();
                    if new_doc_pos != mouse_doc_pos {
                        mouse_doc_pos = new_doc_pos;
                        mouse_idle_since = Some(Instant::now());
                        if hover.visible {
                            hover.hide();
                            redraw = true;
                        }
                        // Immediate diagnostic tooltip.
                        if let Some((line, col)) = new_doc_pos
                            && subsystems.has_lsp()
                            && let Some(doc) = docs.get(active_tab)
                            && let Some(diags) = lsp_state.diagnostics.get(&doc.path)
                        {
                            let l0 = line.saturating_sub(1);
                            let c0 = col.saturating_sub(1);
                            for d in diags {
                                let in_line =
                                    d.start_line <= l0 && l0 <= d.end_line.max(d.start_line);
                                let span_end = d.end_col.max(d.start_col + 1);
                                let in_col = if d.start_line == d.end_line && d.start_line == l0 {
                                    c0 >= d.start_col && c0 < span_end
                                } else if l0 == d.start_line {
                                    c0 >= d.start_col
                                } else if l0 == d.end_line {
                                    c0 < d.end_col
                                } else {
                                    true
                                };
                                if in_line && in_col && !d.message.is_empty() {
                                    hover.text = d.message.clone();
                                    hover.line = line;
                                    hover.col = col;
                                    hover.visible = true;
                                    // Don't also fire LSP hover for this position —
                                    // dedupe by recording it.
                                    last_lsp_hover_pos = Some((line, col));
                                    mouse_idle_since = None;
                                    redraw = true;
                                    break;
                                }
                            }
                        }
                    }
                    continue;
                }
                EditorEvent::MouseReleased { .. } => {
                    if sidebar_dragging {
                        sidebar_dragging = false;
                        let _ = crate::editor::storage::save_text(
                            userdir_path,
                            "session",
                            "sidebar_width",
                            &sidebar_width.to_string(),
                        );
                    }
                    if preview_dragging {
                        preview_dragging = false;
                        let _ = crate::editor::storage::save_text(
                            userdir_path,
                            "session",
                            "preview_split",
                            &preview_split.to_string(),
                        );
                    }
                    if terminal_divider_dragging {
                        terminal_divider_dragging = false;
                        if let Some(h) = terminal_h_override {
                            let _ = crate::editor::storage::save_text(
                                userdir_path,
                                "session",
                                "terminal_height",
                                &h.to_string(),
                            );
                        }
                    }
                    editor_mouse_down = false;
                    tab_dragging = None;
                    editor_sb_dragging = false;
                    terminal_sb_dragging = false;
                    sidebar_sb_dragging = false;
                    // End terminal selection drag; the selection itself
                    // stays visible until dismissed by another click or
                    // the escape / copy key.
                    if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                        inst.sel_dragging = false;
                    }
                    redraw = true;
                    continue;
                }
                EditorEvent::MouseWheel { y, .. } => {
                    let line_h = style.line_height();
                    let scroll_amt = y * line_h * 3.0;
                    // Wheel routes to the terminal panel when the cursor is over it.
                    let over_terminal = subsystems.has_terminal() && terminal.visible && {
                        let (_, wh, _, _) = crate::window::get_window_size();
                        let win_h = wh as f64;
                        let status_h_c = style.font_height + style.padding_y * 2.0;
                        let tab_h_c = if !docs.is_empty() {
                            style.font_height + style.padding_y * 3.0
                        } else {
                            0.0
                        };
                        let terminal_h_c = terminal_h_override
                            .unwrap_or(
                                (win_h * 0.3)
                                    .min(win_h - tab_h_c - status_h_c - 50.0)
                                    .max(80.0),
                            )
                            .min(win_h - tab_h_c - status_h_c - 50.0)
                            .max(80.0);
                        let term_y_c = win_h - terminal_h_c - status_h_c;
                        mouse_y >= term_y_c && mouse_y < win_h - status_h_c
                    };
                    if over_terminal {
                        if let Some(inst) = terminal.terminals.get_mut(terminal.active) {
                            // Positive wheel y walks up into history.
                            let delta = *y * 3.0;
                            let cap = inst.tbuf.history_len() as f64;
                            inst.scrollback_target =
                                (inst.scrollback_target + delta).clamp(0.0, cap);
                        }
                        redraw = true;
                        continue;
                    }
                    if subsystems.has_sidebar() && sidebar_visible && mouse_x < sidebar_width {
                        // Mouse is over the sidebar -- scroll sidebar only.
                        sidebar_scroll_vel -= scroll_amt * 20.0;
                    } else if let Some(doc) = docs.get_mut(active_tab) {
                        // Route wheel to whichever pane the cursor is over
                        // in split preview mode.
                        let over_preview = doc.preview.enabled
                            && doc.preview.rect.w > 0.0
                            && mouse_x >= doc.preview.rect.x
                            && mouse_x < doc.preview.rect.x + doc.preview.rect.w;
                        if over_preview {
                            preview_scroll_vel -= scroll_amt * 20.0;
                        } else {
                            editor_scroll_vel -= scroll_amt * 20.0;
                        }
                    }
                    redraw = true;
                }
                _ => {
                    redraw = true;
                }
            }
        }
}
