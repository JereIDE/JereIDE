use jereide_text::{
    char_index_to_line_col, char_range_substring, delete_char_range, insert_at_char_index,
};

#[test]
fn char_index_to_line_col_empty_string() {
    assert_eq!(char_index_to_line_col("", 0), (1, 1));
}

#[test]
fn char_index_to_line_col_first_char() {
    assert_eq!(char_index_to_line_col("hello", 0), (1, 1));
}

#[test]
fn char_index_to_line_col_second_char() {
    assert_eq!(char_index_to_line_col("hello", 1), (1, 2));
}

#[test]
fn char_index_to_line_col_after_newline() {
    assert_eq!(char_index_to_line_col("a\nb", 2), (2, 1));
}

#[test]
fn char_index_to_line_col_multi_line() {
    assert_eq!(char_index_to_line_col("hello\nworld\n!", 0), (1, 1));
    assert_eq!(char_index_to_line_col("hello\nworld\n!", 6), (2, 1));
    assert_eq!(char_index_to_line_col("hello\nworld\n!", 12), (3, 1));
}

#[test]
fn char_index_to_line_col_beyond_length() {
    assert_eq!(char_index_to_line_col("hi", 10), (1, 3));
}

#[test]
fn char_index_to_line_col_consecutive_newlines() {
    assert_eq!(char_index_to_line_col("a\n\nb", 2), (2, 1));
    assert_eq!(char_index_to_line_col("a\n\nb", 3), (3, 1));
}

#[test]
fn char_range_substring_normal() {
    assert_eq!(char_range_substring("hello world", 0, 5), "hello");
}

#[test]
fn char_range_substring_empty() {
    assert_eq!(char_range_substring("hello", 2, 2), "");
}

#[test]
fn char_range_substring_full() {
    assert_eq!(char_range_substring("hello", 0, 5), "hello");
}

#[test]
fn char_range_substring_middle() {
    assert_eq!(char_range_substring("abcdef", 2, 4), "cd");
}

#[test]
fn char_range_substring_unicode() {
    assert_eq!(char_range_substring("héllo", 0, 2), "hé");
}

#[test]
fn delete_char_range_middle() {
    assert_eq!(delete_char_range("hello world", 5, 11), "hello");
}

#[test]
fn delete_char_range_start() {
    assert_eq!(delete_char_range("hello", 0, 2), "llo");
}

#[test]
fn delete_char_range_end() {
    assert_eq!(delete_char_range("hello", 3, 5), "hel");
}

#[test]
fn delete_char_range_nothing() {
    assert_eq!(delete_char_range("hello", 2, 2), "hello");
}

#[test]
fn delete_char_range_all() {
    assert_eq!(delete_char_range("hello", 0, 5), "");
}

#[test]
fn delete_char_range_unicode() {
    assert_eq!(delete_char_range("héllo", 0, 2), "llo");
}

#[test]
fn insert_at_char_index_beginning() {
    assert_eq!(insert_at_char_index("world", 0, "hello "), "hello world");
}

#[test]
fn insert_at_char_index_end() {
    assert_eq!(insert_at_char_index("hello", 5, " world"), "hello world");
}

#[test]
fn insert_at_char_index_middle() {
    assert_eq!(insert_at_char_index("hworld", 1, "ello"), "helloworld");
}

#[test]
fn insert_at_char_index_empty_insert() {
    assert_eq!(insert_at_char_index("hello", 3, ""), "hello");
}

#[test]
fn insert_at_char_index_unicode() {
    assert_eq!(insert_at_char_index("hllo", 1, "é"), "héllo");
}
