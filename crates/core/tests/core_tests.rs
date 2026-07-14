use jereide_core::{AppState, CurrentView, Tab};

#[test]
fn tab_new_defaults() {
    let tab = Tab::new();
    assert_eq!(tab.text, "");
    assert_eq!(tab.saved_text, "");
    assert_eq!(tab.file_path, None);
    assert_eq!(tab.cursor_line, 1);
    assert_eq!(tab.cursor_col, 1);
    assert!(!tab.is_modified());
}

#[test]
fn tab_with_path_and_content() {
    let tab = Tab::with_path_and_content("/path/to/file.rs".into(), "fn main() {}".into());
    assert_eq!(tab.text, "fn main() {}");
    assert_eq!(tab.saved_text, "fn main() {}");
    assert_eq!(tab.file_path, Some("/path/to/file.rs".into()));
    assert!(!tab.is_modified());
}

#[test]
fn tab_is_modified_after_text_change() {
    let mut tab = Tab::new();
    assert!(!tab.is_modified());
    tab.text = "modified".to_string();
    assert!(tab.is_modified());
}

#[test]
fn tab_mark_saved_resets_modified() {
    let mut tab = Tab::new();
    tab.text = "new content".to_string();
    assert!(tab.is_modified());
    tab.mark_saved();
    assert!(!tab.is_modified());
    assert_eq!(tab.saved_text, "new content");
}

#[test]
fn tab_file_name_with_path() {
    let tab = Tab::with_path_and_content("/home/user/src/main.rs".into(), String::new());
    assert_eq!(tab.file_name(), "main.rs");
}

#[test]
fn tab_file_name_untitled() {
    let tab = Tab::new();
    assert_eq!(tab.file_name(), "Untitled");
}

#[test]
fn tab_file_name_no_extension() {
    let tab = Tab::with_path_and_content("/path/to/Makefile".into(), String::new());
    assert_eq!(tab.file_name(), "Makefile");
}

#[test]
fn tab_file_name_deep_path() {
    let tab = Tab::with_path_and_content(
        "/a/very/deep/nested/directory/file.rs".into(),
        String::new(),
    );
    assert_eq!(tab.file_name(), "file.rs");
}

#[test]
fn app_state_new_has_one_tab() {
    let state = AppState::new();
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.active_tab_index, 0);
    assert_eq!(state.current_view, CurrentView::Code);
}

#[test]
fn app_state_new_tab_creates_and_activates() {
    let mut state = AppState::new();
    let idx = state.new_tab();
    assert_eq!(idx, 1);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.active_tab_index, 1);
}

#[test]
fn app_state_current_tab_returns_active() {
    let mut state = AppState::new();
    state.new_tab();
    state.active_tab_index = 1;
    let tab = state.current_tab();
    assert_eq!(tab.text, "");
}

#[test]
fn app_state_current_tab_mut_modifies_active() {
    let mut state = AppState::new();
    state.current_tab_mut().text = "hello".to_string();
    assert_eq!(state.tabs[0].text, "hello");
}

#[test]
fn app_state_open_file_new() {
    let mut state = AppState::new();
    let idx = state.open_file("/path/to/test.rs".into(), "content".into());
    assert_eq!(idx, 1);
    assert_eq!(state.tabs.len(), 2);
    assert_eq!(state.active_tab_index, 1);
    assert_eq!(state.current_tab().text, "content");
    assert_eq!(
        state.current_tab().file_path,
        Some("/path/to/test.rs".into())
    );
}

#[test]
fn app_state_open_file_already_open_switches_tab() {
    let mut state = AppState::new();
    let idx1 = state.open_file("/path/to/test.rs".into(), "content".into());
    let idx2 = state.open_file("/path/to/test.rs".into(), "content".into());
    assert_eq!(idx1, idx2);
    assert_eq!(state.tabs.len(), 2);
}

#[test]
fn app_state_close_tab_removes() {
    let mut state = AppState::new();
    state.new_tab();
    state.new_tab();
    assert_eq!(state.tabs.len(), 3);
    state.close_tab(1);
    assert_eq!(state.tabs.len(), 2);
}

#[test]
fn app_state_close_last_tab_is_noop() {
    let mut state = AppState::new();
    state.close_tab(0);
    assert_eq!(state.tabs.len(), 1);
}

#[test]
fn app_state_close_tab_adjusts_active_index_down() {
    let mut state = AppState::new();
    state.new_tab();
    state.new_tab();
    state.new_tab();
    state.active_tab_index = 2;
    state.close_tab(0);
    assert_eq!(state.active_tab_index, 1);
}

#[test]
fn app_state_close_tab_clamps_active_index() {
    let mut state = AppState::new();
    state.new_tab();
    state.new_tab();
    state.active_tab_index = 2;
    state.close_tab(2);
    assert_eq!(state.active_tab_index, 1);
}

#[test]
fn app_state_switch_to_view_changes_view() {
    let mut state = AppState::new();
    assert_eq!(state.current_view, CurrentView::Code);
    state.switch_to_view(CurrentView::Command);
    assert_eq!(state.current_view, CurrentView::Command);
}

#[test]
fn app_state_switch_to_view_same_is_noop() {
    let mut state = AppState::new();
    state.switch_to_view(CurrentView::Code);
    assert_eq!(state.current_view, CurrentView::Code);
}

#[test]
fn app_state_is_modified_delegates_to_tab() {
    let mut state = AppState::new();
    assert!(!state.is_modified());
    state.current_tab_mut().text = "changed".to_string();
    assert!(state.is_modified());
}

#[test]
fn app_state_mark_saved_delegates_to_tab() {
    let mut state = AppState::new();
    state.current_tab_mut().text = "changed".to_string();
    assert!(state.is_modified());
    state.mark_saved();
    assert!(!state.is_modified());
}
