#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DiffLineKind {
    Same,
    Added,
    Modified,
}

pub fn diff_lines(old: &str, new: &str) -> (Vec<DiffLineKind>, Vec<usize>) {
    if old == new {
        let n = new.lines().count().max(1);
        return (vec![DiffLineKind::Same; n], Vec::new());
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let m = old_lines.len();
    let n = new_lines.len();

    if n == 0 { return (vec![], Vec::new()); }
    if m == 0 { return (vec![DiffLineKind::Added; n], Vec::new()); }

    let prefix = old_lines.iter().zip(new_lines.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let suffix = old_lines.iter().rev().zip(new_lines.iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(m - prefix)
        .min(n - prefix);

    let mut result = vec![DiffLineKind::Same; n];
    let mut deletions = Vec::new();

    let mid_old_start = prefix;
    let mid_old_end = m - suffix;
    let mid_new_start = prefix;
    let mid_new_end = n - suffix;

    if mid_old_start < mid_old_end || mid_new_start < mid_new_end {
        let old_mid_len = mid_old_end - mid_old_start;
        let new_mid_len = mid_new_end - mid_new_start;

        for j in mid_new_start..mid_new_end {
            result[j] = if j - mid_new_start < old_mid_len {
                DiffLineKind::Modified
            } else {
                DiffLineKind::Added
            };
        }

        if old_mid_len > new_mid_len {
            deletions.push(prefix + new_mid_len);
        }
    }

    (result, deletions)
}

pub fn char_index_to_line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut i = 0;
    for ch in text.chars() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        i += 1;
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
