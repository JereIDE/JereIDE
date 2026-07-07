{
        // Mirror the active editor selection into the X11 PRIMARY selection so
        // middle-click paste (here and in other apps) uses the current
        // selection, matching standard Linux behavior. A no-op on platforms
        // without a primary selection.
        if had_input_events
            && let Some(doc) = docs.get(active_tab)
            && let Some(buf_id) = doc.view.buffer_id
        {
            let coords =
                buffer::with_buffer(buf_id, |b| Ok(b.selections.clone())).unwrap_or_default();
            let key = (buf_id, coords);
            if key != last_primary_key {
                last_primary_key = key;
                let selected = buffer::with_buffer(buf_id, |b| Ok(buffer::get_selected_text(b)))
                    .unwrap_or_default();
                if !selected.is_empty() {
                    crate::window::set_primary_selection_text(&selected);
                }
            }
        }

        // LSP: auto-start for the active file if no transport is running.
        if subsystems.has_lsp()
            && lsp_state.transport_id.is_none()
            && lsp_state.should_attempt_spawn()
            && let Some(doc) = docs.get(active_tab)
            && !doc.path.is_empty()
        {
            try_start_lsp(
                &doc.path,
                &mut lsp_state,
                &lsp_specs,
                userdir,
                config.verbose,
            );
        }

        // Poll background file load. If the thread is done, swap the buffer in.
        if let Some(job) = load_job.as_mut() {
            // Always redraw while a load is active so the progress bar animates.
            redraw = true;
            let finished = job.handle.as_ref().map(|h| h.is_finished()).unwrap_or(true);
            if finished {
                let mut j = load_job.take().unwrap();
                match j.handle.take().unwrap().join() {
                    Ok(Ok(state)) => {
                        let (indent_type, indent_size, _score) =
                            crate::editor::picker::detect_indent(&state.lines, 100, 2);
                        let initial_change_id = state.change_id;
                        let buf_id = buffer::insert_buffer(state);
                        let mut dv = DocView::new();
                        dv.buffer_id = Some(buf_id);
                        dv.indent_size = indent_size;
                        let saved_sig = buffer::with_buffer(buf_id, |b| {
                            Ok(buffer::content_signature(&b.lines))
                        })
                        .unwrap_or(0);
                        docs.push(OpenDoc {
                            view: dv,
                            path: j.path.clone(),
                            name: j.name.clone(),
                            saved_change_id: initial_change_id,
                            saved_signature: saved_sig,
                            indent_type: indent_type.to_string(),
                            indent_size,
                            git_changes: std::collections::HashMap::new(),
                            cached_render: std::sync::Arc::new(Vec::new()),
                            cached_change_id: -1,
                            cached_scroll_y: -1.0,
                            cached_hint_count: 0,
                            cached_rect_w: -1.0,
                            cached_rect_h: -1.0,
                            dirty_cache: std::cell::Cell::new(None),
                            token_cache: std::cell::RefCell::new(
                                crate::editor::open_doc::TokenCache::default(),
                            ),
                            preview: crate::editor::markdown_preview::MarkdownPreviewState::default(
                            ),
                        });
                        active_tab = docs.len() - 1;
                        autoreload.watch(&j.path);
                        remember_recent_file(&mut recent_files, &j.path, userdir_path);
                    }
                    Ok(Err(e)) => {
                        info_message = Some((format!("Load failed: {e}"), Instant::now()));
                    }
                    Err(_) => {
                        info_message = Some(("Load thread panicked".to_string(), Instant::now()));
                    }
                }
            }
        }

        // LSP: poll transport and handle responses.
        if subsystems.has_lsp() {
            if let Some(tid) = lsp_state.transport_id {
                // Request fresh inlay hints whenever the active file
                // changes identity from what `inlay_hints_uri` records.
                if lsp_state.initialized {
                    if let Some(doc) = docs.get(active_tab) {
                        if !doc.path.is_empty() {
                            let ext = doc.path.rsplit('.').next().unwrap_or("");
                            let is_lsp_file = ext_to_lsp_filetype(ext)
                                .map(|ft| ft == lsp_state.filetype)
                                .unwrap_or(false);
                            if is_lsp_file {
                                let uri = path_to_uri(&doc.path);
                                let already_pending =
                                    lsp_state.pending_request_uris.values().any(|u| u == &uri);
                                if lsp_state.inlay_hints_uri != uri && !already_pending {
                                    let line_count = doc
                                        .view
                                        .buffer_id
                                        .and_then(|id| {
                                            buffer::with_buffer(id, |b| Ok(b.lines.len())).ok()
                                        })
                                        .unwrap_or(100);
                                    let req_id = lsp_state.next_id();
                                    lsp_state
                                        .pending_requests
                                        .insert(req_id, "textDocument/inlayHint".to_string());
                                    lsp_state.pending_request_uris.insert(req_id, uri.clone());
                                    let _ = lsp::send_message(
                                        tid,
                                        &lsp_inlay_hint_request(req_id, &uri, 0, line_count),
                                    );
                                    lsp_state.inlay_hints.clear();
                                    lsp_state.inlay_hints_uri = String::new();
                                    for d in &mut docs {
                                        d.cached_change_id = -1;
                                    }
                                }
                            }
                        }
                    }
                }
                // Retry timer for inlay hints while the server is still indexing.
                if let Some(retry_at) = lsp_state.inlay_retry_at {
                    if Instant::now() >= retry_at {
                        lsp_state.inlay_retry_at = None;
                        if let Some(doc) = docs.get(active_tab) {
                            if !doc.path.is_empty() {
                                let ext = doc.path.rsplit('.').next().unwrap_or("");
                                let is_lsp_file = ext_to_lsp_filetype(ext)
                                    .map(|ft| ft == lsp_state.filetype)
                                    .unwrap_or(false);
                                if is_lsp_file {
                                    let uri = path_to_uri(&doc.path);
                                    let line_count = doc
                                        .view
                                        .buffer_id
                                        .and_then(|id| {
                                            buffer::with_buffer(id, |b| Ok(b.lines.len())).ok()
                                        })
                                        .unwrap_or(100);
                                    let req_id = lsp_state.next_request_id;
                                    lsp_state.next_request_id += 1;
                                    lsp_state
                                        .pending_requests
                                        .insert(req_id, "textDocument/inlayHint".to_string());
                                    lsp_state.pending_request_uris.insert(req_id, uri.clone());
                                    let _ = lsp::send_message(
                                        tid,
                                        &lsp_inlay_hint_request(req_id, &uri, 0, line_count),
                                    );
                                }
                            }
                        }
                    }
                }
                if let Ok(poll) = lsp::poll_transport(tid) {
                    for msg in &poll.messages {
                        // Server-to-client `workspace/applyEdit` request: apply the
                        // edit and acknowledge so the server does not block waiting.
                        if msg.get("method").and_then(|m| m.as_str()) == Some("workspace/applyEdit")
                        {
                            let atomic = config.files.atomic_save;
                            let applied = msg
                                .get("params")
                                .and_then(|p| p.get("edit"))
                                .map(|e| apply_lsp_workspace_edit(e, &mut docs, use_git(), atomic))
                                .unwrap_or(0);
                            if applied > 0 {
                                for d in &mut docs {
                                    d.cached_change_id = -1;
                                }
                                crate::window::force_invalidate();
                                redraw = true;
                            }
                            if let (Some(rid), Some(tid)) = (
                                msg.get("id").and_then(|v| v.as_i64()),
                                lsp_state.transport_id,
                            ) {
                                let _ = lsp::send_message(
                                    tid,
                                    &serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": rid,
                                        "result": { "applied": applied > 0 }
                                    }),
                                );
                            }
                            continue;
                        }
                        // Handle initialize response.
                        if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("initialize")
                            {
                                lsp_state.pending_requests.remove(&id);
                                lsp_state.initialized = true;
                                lsp_state.note_spawn_success();
                                // Send initialized notification.
                                let _ = lsp::send_message(
                                    tid,
                                    &serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "method": "initialized",
                                        "params": {}
                                    }),
                                );
                                // Send didOpen only for files matching the LSP filetype.
                                for doc in &docs {
                                    if doc.path.is_empty() {
                                        continue;
                                    }
                                    let ext = doc.path.rsplit('.').next().unwrap_or("");
                                    let Some(ft) = ext_to_lsp_filetype(ext) else {
                                        continue;
                                    };
                                    if ft != lsp_state.filetype {
                                        continue;
                                    }
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let text =
                                            buffer::with_buffer(buf_id, |b| Ok(b.lines.join("")))
                                                .unwrap_or_default();
                                        let uri = path_to_uri(&doc.path);
                                        let _ = lsp::send_message(
                                            tid,
                                            &lsp_did_open(&uri, &lsp_state.filetype, &text),
                                        );
                                    }
                                }
                                // Request inlay hints only for the active file if it matches LSP.
                                if let Some(doc) = docs.get(active_tab) {
                                    let ext = doc.path.rsplit('.').next().unwrap_or("");
                                    if ext_to_lsp_filetype(ext)
                                        .map(|ft| ft == lsp_state.filetype)
                                        .unwrap_or(false)
                                    {
                                        let uri = path_to_uri(&doc.path);
                                        let line_count = doc
                                            .view
                                            .buffer_id
                                            .and_then(|id| {
                                                buffer::with_buffer(id, |b| Ok(b.lines.len())).ok()
                                            })
                                            .unwrap_or(100);
                                        let req_id = lsp_state.next_id();
                                        lsp_state
                                            .pending_requests
                                            .insert(req_id, "textDocument/inlayHint".to_string());
                                        lsp_state.pending_request_uris.insert(req_id, uri.clone());
                                        let _ = lsp::send_message(
                                            tid,
                                            &lsp_inlay_hint_request(req_id, &uri, 0, line_count),
                                        );
                                    }
                                }
                            }

                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/inlayHint")
                            {
                                lsp_state.pending_requests.remove(&id);
                                let req_uri = lsp_state
                                    .pending_request_uris
                                    .remove(&id)
                                    .unwrap_or_default();
                                let active_uri = docs
                                    .get(active_tab)
                                    .filter(|d| !d.path.is_empty())
                                    .map(|d| path_to_uri(&d.path))
                                    .unwrap_or_default();
                                if !req_uri.is_empty() && req_uri != active_uri {
                                    continue;
                                }
                                if let Some(result) = msg.get("result").and_then(|r| r.as_array()) {
                                    let mut new_hints: Vec<InlayHint> =
                                        Vec::with_capacity(result.len());
                                    for hint in result {
                                        let line = hint
                                            .get("position")
                                            .and_then(|p| p.get("line"))
                                            .and_then(|l| l.as_i64())
                                            .unwrap_or(0)
                                            as usize;
                                        let col = hint
                                            .get("position")
                                            .and_then(|p| p.get("character"))
                                            .and_then(|c| c.as_i64())
                                            .unwrap_or(0)
                                            as usize;
                                        let label = if let Some(s) =
                                            hint.get("label").and_then(|l| l.as_str())
                                        {
                                            s.to_string()
                                        } else if let Some(parts) =
                                            hint.get("label").and_then(|l| l.as_array())
                                        {
                                            parts
                                                .iter()
                                                .filter_map(|p| {
                                                    p.get("value").and_then(|v| v.as_str())
                                                })
                                                .collect::<Vec<_>>()
                                                .join("")
                                        } else {
                                            continue;
                                        };
                                        let padding_left = hint
                                            .get("paddingLeft")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(true);
                                        let padding_right = hint
                                            .get("paddingRight")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(false);
                                        let mut display = label;
                                        if padding_left {
                                            display = format!(" {display}");
                                        }
                                        if padding_right {
                                            display = format!("{display} ");
                                        }
                                        new_hints.push(InlayHint {
                                            line,
                                            col,
                                            label: display,
                                        });
                                    }
                                    if new_hints.is_empty() {
                                        if lsp_state.inlay_retry_count < 20 {
                                            lsp_state.inlay_retry_at = Some(
                                                Instant::now() + std::time::Duration::from_secs(2),
                                            );
                                            lsp_state.inlay_retry_count += 1;
                                        }
                                    } else {
                                        // Detect any difference in positions or
                                        // labels — count alone is not enough.
                                        // After a small edit the number of
                                        // hints often stays identical while
                                        // every hint's `col` shifts; comparing
                                        // only `len()` would let stale render
                                        // tokens leak through and the inlays
                                        // would render at their previous
                                        // positions until the next structural
                                        // change.
                                        let uri_changed = lsp_state.inlay_hints_uri != req_uri;
                                        let content_changed = uri_changed
                                            || lsp_state.inlay_hints.len() != new_hints.len()
                                            || lsp_state
                                                .inlay_hints
                                                .iter()
                                                .zip(new_hints.iter())
                                                .any(|(a, b)| {
                                                    a.line != b.line
                                                        || a.col != b.col
                                                        || a.label != b.label
                                                });
                                        lsp_state.inlay_hints = new_hints;
                                        lsp_state.inlay_hints_uri = req_uri.clone();
                                        lsp_state.inlay_retry_count = 0;
                                        lsp_state.inlay_retry_at = None;
                                        if content_changed {
                                            pending_render_cache = None;
                                            for d in &mut docs {
                                                d.cached_change_id = -1;
                                            }
                                            crate::window::force_invalidate();
                                        }
                                    }
                                    redraw = true;
                                }
                            }

                            // Handle completion response.
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/completion")
                            {
                                lsp_state.pending_requests.remove(&id);
                                // Drop responses for any request older than the
                                // latest one we sent — LSP servers may answer
                                // out of order, and a slow stale reply (with a
                                // shorter prefix) would otherwise clobber a
                                // fresher list. Mirrors the inlay-hint
                                // late-response gate.
                                if id != completion.latest_request_id {
                                    continue;
                                }
                                // Re-derive the word-prefix at the cursor RIGHT
                                // NOW (the user may have typed more characters
                                // between the request being sent and this
                                // reply). The LSP server already filters by
                                // its own prefix-snapshot; we re-filter
                                // client-side so the popup never shows an
                                // item that doesn't match the current word.
                                let prefix_now: String = docs
                                    .get(active_tab)
                                    .and_then(|d| d.view.buffer_id)
                                    .and_then(|bid| {
                                        buffer::with_buffer(bid, |b| {
                                            let l = *b.selections.get(2).unwrap_or(&1);
                                            let c = *b.selections.get(3).unwrap_or(&1);
                                            let line = b
                                                .lines
                                                .get(l - 1)
                                                .map(String::as_str)
                                                .unwrap_or("");
                                            let chars: Vec<char> = line.chars().collect();
                                            let col = (c - 1).min(chars.len());
                                            let mut start = col;
                                            while start > 0 {
                                                let ch = chars[start - 1];
                                                if ch.is_alphanumeric() || ch == '_' {
                                                    start -= 1;
                                                } else {
                                                    break;
                                                }
                                            }
                                            Ok(chars[start..col].iter().collect::<String>())
                                        })
                                        .ok()
                                    })
                                    .unwrap_or_default();
                                let mut items: Vec<(String, String, String)> = Vec::new();
                                let result = msg.get("result");
                                // result can be an array or {items: [...]}.
                                let item_arr = result
                                    .and_then(|r| {
                                        r.as_array().cloned().or_else(|| {
                                            r.get("items").and_then(|v| v.as_array()).cloned()
                                        })
                                    })
                                    .unwrap_or_default();
                                for item in item_arr.iter() {
                                    let label = item
                                        .get("label")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !prefix_now.is_empty() && !label.starts_with(&prefix_now) {
                                        continue;
                                    }
                                    let detail = item
                                        .get("detail")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let insert_text = item
                                        .get("insertText")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| {
                                            item.get("textEdit")
                                                .and_then(|te| te.get("newText"))
                                                .and_then(|v| v.as_str())
                                        })
                                        .unwrap_or(&label)
                                        .to_string();
                                    items.push((label, detail, insert_text));
                                    if items.len() >= 20 {
                                        break;
                                    }
                                }
                                if !items.is_empty() && !cmdview_active && !palette_active {
                                    completion.items = items;
                                    completion.selected = 0;
                                    completion.scroll_offset = 0;
                                    completion.visible = true;
                                } else {
                                    completion.hide();
                                }
                                redraw = true;
                            }

                            // Handle hover response.
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/hover")
                            {
                                lsp_state.pending_requests.remove(&id);
                                let contents = msg.get("result").and_then(|r| r.get("contents"));
                                let text = contents
                                    .and_then(|c| {
                                        // MarkupContent: {kind, value}
                                        c.get("value")
                                            .and_then(|v| v.as_str())
                                            .map(String::from)
                                            .or_else(|| {
                                                // Plain string.
                                                c.as_str().map(String::from)
                                            })
                                    })
                                    .unwrap_or_default();
                                if !text.is_empty() {
                                    hover.text = text;
                                    hover.visible = true;
                                } else {
                                    hover.hide();
                                }
                                redraw = true;
                            }

                            // Handle go-to-definition response.
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/definition")
                            {
                                lsp_state.pending_requests.remove(&id);
                                let result = msg.get("result");
                                // result can be Location, Location[], or null.
                                let loc = result.and_then(|r| {
                                    if r.is_array() {
                                        r.as_array().and_then(|a| a.first())
                                    } else if r.is_object() {
                                        Some(r)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(location) = loc {
                                    let target_uri =
                                        location.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                                    let target_line = location
                                        .get("range")
                                        .and_then(|r| r.get("start"))
                                        .and_then(|s| s.get("line"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        as usize
                                        + 1;
                                    let target_col = location
                                        .get("range")
                                        .and_then(|r| r.get("start"))
                                        .and_then(|s| s.get("character"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        as usize
                                        + 1;
                                    let target_path = uri_to_path(target_uri);
                                    if !target_path.is_empty() {
                                        // Open or switch to file.
                                        let existing =
                                            docs.iter().position(|d| d.path == target_path);
                                        let tab_idx = if let Some(idx) = existing {
                                            idx
                                        } else {
                                            open_file_into(&target_path, &mut docs, use_git());
                                            autoreload.watch(&target_path);
                                            remember_recent_file(
                                                &mut recent_files,
                                                &target_path,
                                                userdir_path,
                                            );
                                            docs.len() - 1
                                        };
                                        active_tab = tab_idx;
                                        // Set cursor to target position.
                                        if let Some(doc) = docs.get(active_tab) {
                                            if let Some(buf_id) = doc.view.buffer_id {
                                                let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                    let line =
                                                        target_line.min(b.lines.len()).max(1);
                                                    let max_col = char_count(
                                                        b.lines[line - 1].trim_end_matches('\n'),
                                                    ) + 1;
                                                    let col = target_col.min(max_col);
                                                    b.selections = vec![line, col, line, col];
                                                    Ok(())
                                                });
                                            }
                                        }
                                    }
                                }
                                redraw = true;
                            }

                            // Handle document-formatting response (manual or on-save).
                            {
                                let fmt_method = lsp_state.pending_requests.get(&id).cloned();
                                if matches!(
                                    fmt_method.as_deref(),
                                    Some(
                                        "textDocument/formatting" | "textDocument/formatting@save"
                                    )
                                ) {
                                    lsp_state.pending_requests.remove(&id);
                                    let save_after = fmt_method.as_deref()
                                        == Some("textDocument/formatting@save");
                                    let req_uri = lsp_state
                                        .pending_request_uris
                                        .remove(&id)
                                        .unwrap_or_default();
                                    if let Some(edits) =
                                        msg.get("result").and_then(|r| r.as_array())
                                        && let Some(idx) = docs.iter().position(|d| {
                                            !d.path.is_empty() && path_to_uri(&d.path) == req_uri
                                        })
                                        && let Some(buf_id) = docs[idx].view.buffer_id
                                    {
                                        let changed = buffer::with_buffer_mut(buf_id, |b| {
                                            Ok(apply_lsp_text_edits(b, edits))
                                        })
                                        .unwrap_or(false);
                                        if changed {
                                            docs[idx].cached_change_id = -1;
                                            docs[idx].cached_render =
                                                std::sync::Arc::new(Vec::new());
                                            if save_after {
                                                let path = docs[idx].path.clone();
                                                let atomic = config.files.atomic_save;
                                                if let Ok(Ok(cid)) =
                                                    buffer::with_buffer(buf_id, |b| {
                                                        Ok(buffer::save_file(
                                                            b, &path, b.crlf, atomic,
                                                        )
                                                        .map(|()| b.change_id))
                                                    })
                                                {
                                                    docs[idx].saved_change_id = cid;
                                                    docs[idx].saved_signature =
                                                        buffer::with_buffer(buf_id, |b| {
                                                            Ok(buffer::content_signature(&b.lines))
                                                        })
                                                        .unwrap_or(0);
                                                }
                                            }
                                            redraw = true;
                                        }
                                    }
                                }
                            }

                            // Handle rename response (WorkspaceEdit).
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/rename")
                            {
                                lsp_state.pending_requests.remove(&id);
                                if let Some(result) = msg.get("result").filter(|r| !r.is_null()) {
                                    let atomic = config.files.atomic_save;
                                    let n = apply_lsp_workspace_edit(
                                        result,
                                        &mut docs,
                                        use_git(),
                                        atomic,
                                    );
                                    info_message = Some((
                                        format!("Renamed across {n} file(s)"),
                                        Instant::now(),
                                    ));
                                    for d in &mut docs {
                                        d.cached_change_id = -1;
                                    }
                                    crate::window::force_invalidate();
                                } else {
                                    info_message = Some((
                                        "Rename produced no changes".to_string(),
                                        Instant::now(),
                                    ));
                                }
                                redraw = true;
                            }

                            // Handle code-action response: collect into the picker.
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/codeAction")
                            {
                                lsp_state.pending_requests.remove(&id);
                                code_actions.clear();
                                if let Some(arr) = msg.get("result").and_then(|r| r.as_array()) {
                                    for a in arr {
                                        let title = a
                                            .get("title")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if !title.is_empty() {
                                            code_actions.push((title, a.clone()));
                                        }
                                    }
                                }
                                if code_actions.is_empty() {
                                    info_message = Some((
                                        "No code actions available".to_string(),
                                        Instant::now(),
                                    ));
                                    code_action_active = false;
                                } else {
                                    code_action_selected = 0;
                                    code_action_active = true;
                                }
                                redraw = true;
                            }

                            // Handle signature-help response.
                            if lsp_state.pending_requests.get(&id).map(|s| s.as_str())
                                == Some("textDocument/signatureHelp")
                            {
                                lsp_state.pending_requests.remove(&id);
                                let result = msg.get("result");
                                let sig = result
                                    .and_then(|r| r.get("signatures"))
                                    .and_then(|s| s.as_array())
                                    .and_then(|arr| {
                                        let active = result
                                            .and_then(|r| r.get("activeSignature"))
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0)
                                            as usize;
                                        arr.get(active).or_else(|| arr.first())
                                    })
                                    .and_then(|s| s.get("label"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if sig.is_empty() {
                                    signature_help.hide();
                                } else {
                                    signature_help.text = sig;
                                    signature_help.visible = true;
                                }
                                redraw = true;
                            }

                            // Handle implementation/typeDefinition/references responses.
                            // These return the same Location/Location[] format as definition.
                            let method_str = lsp_state.pending_requests.get(&id).cloned();
                            if matches!(
                                method_str.as_deref(),
                                Some(
                                    "textDocument/implementation"
                                        | "textDocument/typeDefinition"
                                        | "textDocument/references"
                                )
                            ) {
                                lsp_state.pending_requests.remove(&id);
                                let result = msg.get("result");
                                let loc = result.and_then(|r| {
                                    if r.is_array() {
                                        r.as_array().and_then(|a| a.first())
                                    } else if r.is_object() {
                                        Some(r)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(location) = loc {
                                    let target_uri =
                                        location.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                                    let target_line = location
                                        .get("range")
                                        .and_then(|r| r.get("start"))
                                        .and_then(|s| s.get("line"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        as usize
                                        + 1;
                                    let target_col = location
                                        .get("range")
                                        .and_then(|r| r.get("start"))
                                        .and_then(|s| s.get("character"))
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0)
                                        as usize
                                        + 1;
                                    let target_path = uri_to_path(target_uri);
                                    if !target_path.is_empty() {
                                        let existing =
                                            docs.iter().position(|d| d.path == target_path);
                                        let tab_idx = if let Some(idx) = existing {
                                            idx
                                        } else {
                                            open_file_into(&target_path, &mut docs, use_git());
                                            autoreload.watch(&target_path);
                                            remember_recent_file(
                                                &mut recent_files,
                                                &target_path,
                                                userdir_path,
                                            );
                                            docs.len() - 1
                                        };
                                        active_tab = tab_idx;
                                        if let Some(doc) = docs.get(active_tab) {
                                            if let Some(buf_id) = doc.view.buffer_id {
                                                let _ = buffer::with_buffer_mut(buf_id, |b| {
                                                    let line =
                                                        target_line.min(b.lines.len()).max(1);
                                                    let max_col = char_count(
                                                        b.lines[line - 1].trim_end_matches('\n'),
                                                    ) + 1;
                                                    let col = target_col.min(max_col);
                                                    b.selections = vec![line, col, line, col];
                                                    Ok(())
                                                });
                                            }
                                        }
                                    }
                                }
                                redraw = true;
                            }
                        }
                        // Handle publishDiagnostics.
                        if msg.get("method").and_then(|v| v.as_str())
                            == Some("textDocument/publishDiagnostics")
                        {
                            if let Some(params) = msg.get("params") {
                                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                                    let path = uri_to_path(uri);
                                    let diags: Vec<Diagnostic> = params
                                        .get("diagnostics")
                                        .and_then(|v| v.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .map(|d| {
                                                    let range = d.get("range");
                                                    let start = range.and_then(|r| r.get("start"));
                                                    let end = range.and_then(|r| r.get("end"));
                                                    Diagnostic {
                                                        start_line: start
                                                            .and_then(|s| s.get("line"))
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0)
                                                            as usize,
                                                        start_col: start
                                                            .and_then(|s| s.get("character"))
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0)
                                                            as usize,
                                                        end_line: end
                                                            .and_then(|s| s.get("line"))
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0)
                                                            as usize,
                                                        end_col: end
                                                            .and_then(|s| s.get("character"))
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(0)
                                                            as usize,
                                                        severity: d
                                                            .get("severity")
                                                            .and_then(|v| v.as_u64())
                                                            .unwrap_or(1)
                                                            as u8,
                                                        message: d
                                                            .get("message")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("")
                                                            .to_string(),
                                                    }
                                                })
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    // A cleared-diagnostics publish (empty list)
                                    // drops the entry instead of leaving an empty
                                    // vec behind, so the map does not grow with
                                    // every file the server has ever reported on.
                                    if diags.is_empty() {
                                        lsp_state.diagnostics.remove(&path);
                                    } else {
                                        lsp_state.diagnostics.insert(path, diags);
                                    }
                                    redraw = true;
                                }
                            }
                        }
                    }
                    if !poll.running {
                        lsp::remove_transport(tid);
                        lsp_state.note_spawn_failure();
                        lsp_state.transport_id = None;
                        lsp_state.initialized = false;
                    }
                }
            }
        }

        // Detect any buffer mutation on the active doc by watching
        // `change_id`. The typing path used to flip `last_change` itself,
        // but every other edit route (paste, undo, redo, format-document,
        // multi-cursor delete, snippet apply, find-and-replace) bypassed
        // that flag, so inlay hints went stale until the next keystroke.
        // Polling the change counter per frame catches all of them in one
        // place.
        //
        // Word-index dirty marking runs unconditionally (no LSP needed);
        // the LSP inlay-hint / didChange logic below this block is gated
        // on `has_lsp`.
        if let Some(buf_id) = docs.get(active_tab).and_then(|d| d.view.buffer_id) {
            let cur = buffer::with_buffer(buf_id, |b| Ok(b.change_id)).unwrap_or(0);
            let prev = word_index.last_seen_change_id.get(&buf_id).copied();
            match prev {
                None => {
                    word_index.last_seen_change_id.insert(buf_id, cur);
                    word_index.dirty = true;
                }
                Some(p) if p != cur => {
                    word_index.last_seen_change_id.insert(buf_id, cur);
                    word_index.dirty = true;
                }
                _ => {}
            }
        }

        if subsystems.has_lsp() && lsp_state.transport_id.is_some() && lsp_state.initialized {
            if let Some(doc) = docs.get(active_tab) {
                if !doc.path.is_empty() {
                    let ext = doc.path.rsplit('.').next().unwrap_or("");
                    let is_lsp_file = ext_to_lsp_filetype(ext)
                        .map(|ft| ft == lsp_state.filetype)
                        .unwrap_or(false);
                    if is_lsp_file {
                        if let Some(buf_id) = doc.view.buffer_id {
                            let cur = buffer::with_buffer(buf_id, |b| Ok(b.change_id)).unwrap_or(0);
                            let uri = path_to_uri(&doc.path);
                            let prev = lsp_state.last_seen_change_id.get(&uri).copied();
                            match prev {
                                None => {
                                    lsp_state.last_seen_change_id.insert(uri, cur);
                                }
                                Some(p) if p != cur => {
                                    lsp_state.last_seen_change_id.insert(uri.clone(), cur);
                                    lsp_state.last_change = Some(Instant::now());
                                    lsp_state.pending_change_uri = Some(uri);
                                    lsp_state.pending_change_version += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // LSP: flush debounced didChange after 300ms of no changes.
        if subsystems.has_lsp() {
            if let Some(last) = lsp_state.last_change {
                if last.elapsed().as_millis() >= 300 {
                    if let (Some(tid), Some(uri)) =
                        (lsp_state.transport_id, lsp_state.pending_change_uri.take())
                    {
                        if lsp_state.initialized {
                            // Read current buffer text for the file.
                            let file_path = uri_to_path(&uri);
                            if let Some(doc) = docs.iter().find(|d| d.path == file_path) {
                                let ext = doc.path.rsplit('.').next().unwrap_or("");
                                let is_lsp_file = ext_to_lsp_filetype(ext)
                                    .map(|ft| ft == lsp_state.filetype)
                                    .unwrap_or(false);
                                if is_lsp_file {
                                    if let Some(buf_id) = doc.view.buffer_id {
                                        let text =
                                            buffer::with_buffer(buf_id, |b| Ok(b.lines.join("")))
                                                .unwrap_or_default();
                                        let _ = lsp::send_message(
                                            tid,
                                            &lsp_did_change(
                                                &uri,
                                                lsp_state.pending_change_version,
                                                &text,
                                            ),
                                        );
                                        // Re-request inlay hints after change is flushed.
                                        let line_count =
                                            buffer::with_buffer(buf_id, |b| Ok(b.lines.len()))
                                                .unwrap_or(100);
                                        let req_id = lsp_state.next_id();
                                        lsp_state
                                            .pending_requests
                                            .insert(req_id, "textDocument/inlayHint".to_string());
                                        lsp_state.pending_request_uris.insert(req_id, uri.clone());
                                        let _ = lsp::send_message(
                                            tid,
                                            &lsp_inlay_hint_request(req_id, &uri, 0, line_count),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    lsp_state.last_change = None;
                }
            }
        }

        // LSP: fire a deferred `textDocument/hover` after the mouse
        // has been still for ~600ms over a code position with no
        // diagnostic under it. Keeps the LSP unspammed while the
        // cursor moves; surfaces type / doc info as a tooltip once
        // the user pauses.
        if subsystems.has_lsp()
            && !hover.visible
            && let Some(idle_since) = mouse_idle_since
            && let Some((line, col)) = mouse_doc_pos
            && idle_since.elapsed() >= std::time::Duration::from_millis(600)
            && last_lsp_hover_pos != Some((line, col))
        {
            mouse_idle_since = None;
            last_lsp_hover_pos = Some((line, col));
            if let Some(doc) = docs.get(active_tab)
                && let Some(tid) = lsp_state.transport_id
                && lsp_state.initialized
                && !doc.path.is_empty()
                && doc.view.buffer_id.is_some()
            {
                let uri = path_to_uri(&doc.path);
                let req_id = lsp_state.next_id();
                lsp_state
                    .pending_requests
                    .insert(req_id, "textDocument/hover".to_string());
                let _ = lsp::send_message(tid, &lsp_hover_request(req_id, &uri, line - 1, col - 1));
                hover.line = line;
                hover.col = col;
            }
        }

        // Terminal: poll/drain/reap every pty each frame regardless of panel
        // visibility, so a shell that exits or floods output while the panel
        // is hidden is still reaped and its pty kept drained. Only repaints
        // are gated on visibility.
        if subsystems.has_terminal() {
            let mut dead_indices = Vec::new();
            for (i, inst) in terminal.terminals.iter_mut().enumerate() {
                inst.inner.poll();
                if !inst.inner.running {
                    dead_indices.push(i);
                } else {
                    // Drain the pty up to a per-frame byte budget so a burst
                    // (a build, `cat bigfile`) shows up in one frame instead of
                    // trickling at 4 KiB/frame, while still yielding to the UI
                    // within the budget. The pty back-pressures the child once
                    // its kernel buffer fills, so nothing is lost.
                    let mut remaining: usize = 256 * 1024;
                    while remaining > 0 {
                        match inst.inner.read(remaining.min(64 * 1024)) {
                            Some(data) if !data.is_empty() => {
                                remaining = remaining.saturating_sub(data.len());
                                inst.tbuf.process_output(&data);
                                if terminal.visible {
                                    redraw = true;
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
            // Remove dead terminals in reverse order.
            for i in dead_indices.into_iter().rev() {
                terminal.terminals[i].inner.cleanup();
                terminal.terminals.remove(i);
                if terminal.visible {
                    redraw = true;
                }
            }
            if terminal.terminals.is_empty() {
                let was_visible = terminal.visible;
                terminal.visible = false;
                terminal.focused = false;
                terminal.active = 0;
                if was_visible {
                    // Panel just went away -- force a native repaint so the
                    // editor content reclaims the vacated strip in the
                    // same frame instead of waiting for the next event.
                    crate::window::force_invalidate();
                }
            } else if terminal.active >= terminal.terminals.len() {
                terminal.active = terminal.terminals.len() - 1;
            }
        }

        // Git: surface results of async mutations (push/pull/commit/stash) and
        // apply async diff results to their docs. These run on worker threads,
        // so the render loop never blocks on git network or fork/exec I/O.
        for m in crate::editor::git::drain_finished_mutations() {
            let detail = m.result.stderr.trim();
            let msg = if m.result.ok {
                format!("{}: done", m.label)
            } else if !detail.is_empty() {
                format!("{}: {detail}", m.label)
            } else {
                format!("{} failed", m.label)
            };
            info_message = Some((msg, Instant::now()));
            // A mutation can change the working-tree baseline, so refresh the
            // diff gutters of the open docs against the new git state.
            for doc in &docs {
                if !doc.path.is_empty() {
                    crate::editor::git::start_diff(&doc.path);
                }
            }
            redraw = true;
        }
        for d in crate::editor::git::drain_diffs() {
            if let Some(doc) = docs.iter_mut().find(|doc| doc.path == d.path) {
                doc.git_changes = d.changes;
                redraw = true;
            }
        }

        // Project search/replace: pick up async grep results for the active
        // panel. run_project_search is non-blocking and refreshes in the
        // background; apply the latest results when they differ.
        if subsystems.has_find_in_files() {
            if project_search_active {
                let r = project_search::run_project_search(
                    &project_search_query,
                    &project_root,
                    project_use_regex,
                    project_whole_word,
                    project_case_insensitive,
                );
                if r != project_search_results {
                    project_search_results = r;
                    if project_search_selected >= project_search_results.len() {
                        project_search_selected = project_search_results.len().saturating_sub(1);
                    }
                    redraw = true;
                }
            }
            if project_replace_active {
                let r = project_search::run_project_search(
                    &project_replace_search,
                    &project_root,
                    project_use_regex,
                    project_whole_word,
                    project_case_insensitive,
                );
                if r != project_replace_results {
                    project_replace_results = r;
                    if project_replace_selected >= project_replace_results.len() {
                        project_replace_selected = project_replace_results.len().saturating_sub(1);
                    }
                    redraw = true;
                }
            }
        }

        // Project replace-all: apply the result of the background sed job.
        if let Some(job) = replace_job.take() {
            if job.is_finished() {
                let count = job.join().unwrap_or(0);
                info_message = Some((
                    format!("Replaced {count} occurrences across project"),
                    Instant::now(),
                ));
                // Reload any open files the replace may have changed.
                for doc in &mut docs {
                    if let Some(buf_id) = doc.view.buffer_id {
                        if !doc.path.is_empty() {
                            let _ = buffer::with_buffer_mut(buf_id, |b| {
                                let mut fresh = buffer::default_buffer_state();
                                if buffer::load_file(&mut fresh, &doc.path).is_ok() {
                                    b.lines = fresh.lines;
                                    b.change_id += 1;
                                }
                                Ok(())
                            });
                        }
                    }
                }
                redraw = true;
            } else {
                replace_job = Some(job);
            }
        }

        // Git status panel: apply the result of the background refresh.
        if let Some(job) = git_status_job.take() {
            if job.is_finished() {
                git_status_entries = job.join().unwrap_or_default();
                if git_status_selected >= git_status_entries.len() {
                    git_status_selected = git_status_entries.len().saturating_sub(1);
                }
                redraw = true;
            } else {
                git_status_job = Some(job);
            }
        }

        // Git blame overlay: apply the background result if still wanted.
        if let Some(job) = git_blame_job.take() {
            if job.is_finished() {
                let lines = job.join().unwrap_or_default();
                if git_blame_active {
                    git_blame_lines = lines;
                }
                redraw = true;
            } else {
                git_blame_job = Some(job);
            }
        }

        // Git log panel: apply the background result.
        if let Some(job) = git_log_job.take() {
            if job.is_finished() {
                git_log_entries = job.join().unwrap_or_default();
                if git_log_selected >= git_log_entries.len() {
                    git_log_selected = git_log_entries.len().saturating_sub(1);
                }
                redraw = true;
            } else {
                git_log_job = Some(job);
            }
        }

        // Update check: surface the version comparison when curl returns.
        if let Some(job) = update_check_job.take() {
            if job.is_finished() {
                if let Ok(msg) = job.join() {
                    info_message = Some((msg, Instant::now()));
                }
                redraw = true;
            } else {
                update_check_job = Some(job);
            }
        }
}
