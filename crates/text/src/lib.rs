/// For the line/column indicator.
pub fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (ci, ch) in text.chars().enumerate() {
        if ci >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line + 1, col + 1)
}

pub fn char_range_substring(text: &str, start_char: usize, end_char: usize) -> String {
    text.chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect()
}

pub fn delete_char_range(text: &str, start_char: usize, end_char: usize) -> String {
    text.chars()
        .enumerate()
        .filter(|(i, _)| *i < start_char || *i >= end_char)
        .map(|(_, c)| c)
        .collect()
}

pub fn insert_at_char_index(text: &str, char_index: usize, insert: &str) -> String {
    let before: String = text.chars().take(char_index).collect();
    let after: String = text.chars().skip(char_index).collect();
    format!("{}{}{}", before, insert, after)
}
