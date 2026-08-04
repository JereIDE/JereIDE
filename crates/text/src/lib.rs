/// Number of lines, counting each `\n` as the end of a line.
pub fn count_lines(text: &str) -> usize {
    text.chars().filter(|&c| c == '\n').count() + 1
}

/// Char index of the start of the given 1-based line. Clamps to the end of the text.
pub fn line_start_char_index(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut current = 1;
    for (char_idx, ch) in text.chars().enumerate() {
        if ch == '\n' {
            current += 1;
            if current == line {
                return char_idx + 1;
            }
        }
    }
    text.chars().count()
}

/// For the line/column indicator.
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

pub fn find_matches(
    text: &str,
    query: &str,
    match_case: bool,
    whole_word: bool,
) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let qchars: Vec<char> = query.chars().collect();
    if qchars.is_empty() || chars.len() < qchars.len() {
        return Vec::new();
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let eq = |a: char, b: char| {
        if match_case {
            a == b
        } else {
            a.to_lowercase().eq(b.to_lowercase())
        }
    };
    let mut result = Vec::new();
    let n = chars.len();
    let m = qchars.len();
    let mut i = 0;
    while i + m <= n {
        let mut matched = true;
        for k in 0..m {
            if !eq(chars[i + k], qchars[k]) {
                matched = false;
                break;
            }
        }
        if matched {
            let end = i + m;
            let before_ok = i == 0 || !is_word(chars[i - 1]);
            let after_ok = end >= n || !is_word(chars[end]);
            if !whole_word || (before_ok && after_ok) {
                result.push((i, end));
            }
        }
        i += 1;
    }
    result
}

#[cfg(test)]
mod find_matches_tests {
    use super::find_matches;

    #[test]
    fn finds_all_occurrences() {
        assert_eq!(
            find_matches("aaa", "a", true, false),
            vec![(0, 1), (1, 2), (2, 3)]
        );
    }

    #[test]
    fn empty_query_returns_nothing() {
        assert!(find_matches("hello", "", true, false).is_empty());
    }

    #[test]
    fn empty_text_returns_nothing() {
        assert!(find_matches("", "a", true, false).is_empty());
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(find_matches("abc", "xyz", true, false).is_empty());
    }

    #[test]
    fn case_sensitive_respects_case() {
        assert_eq!(
            find_matches("Hello hello HELLO", "hello", true, false),
            vec![(6, 11)]
        );
    }

    #[test]
    fn case_insensitive_matches_any_case() {
        assert_eq!(
            find_matches("Hello hello HELLO", "hello", false, false).len(),
            3
        );
    }

    #[test]
    fn whole_word_requires_boundaries() {
        assert_eq!(
            find_matches("cat catalog scatter", "cat", true, true),
            vec![(0, 3)]
        );
    }

    #[test]
    fn whole_word_allows_underscore_as_word_char() {
        assert!(find_matches("my_var", "var", true, true).is_empty());
    }

    #[test]
    fn overlapping_matches_are_kept() {
        assert_eq!(find_matches("aaa", "aa", true, false), vec![(0, 2), (1, 3)]);
    }

    #[test]
    fn whole_word_at_text_boundaries() {
        assert_eq!(
            find_matches("hello world", "hello", true, true),
            vec![(0, 5)]
        );
        assert_eq!(
            find_matches("hello world", "world", true, true),
            vec![(6, 11)]
        );
    }
}
