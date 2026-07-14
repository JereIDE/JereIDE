use jereide_syntax::SyntaxHighlighter;

#[test]
fn syntax_highlighter_empty_text() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job = hl.highlight("");
    assert_eq!(job.text, "");
}

#[test]
fn syntax_highlighter_plain_text() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job = hl.highlight("hello world");
    assert_eq!(job.text, "hello world");
}

#[test]
fn syntax_highlighter_rust_keyword() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("rs"));
    let job = hl.highlight("fn main() {}");
    assert_eq!(job.text, "fn main() {}");
}

#[test]
fn syntax_highlighter_cache_same_input() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job1 = hl.highlight("hello");
    let job2 = hl.highlight("hello");
    assert_eq!(job1.text, job2.text);
}

#[test]
fn syntax_highlighter_cache_invalidated_on_change() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    hl.highlight("hello");
    let job = hl.highlight("world");
    assert_eq!(job.text, "world");
}

#[test]
fn syntax_highlighter_switching_extension() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("rs"));
    let job_rs = hl.highlight("fn main() {}");
    assert_eq!(job_rs.text, "fn main() {}");

    let mut hl2 = SyntaxHighlighter::new(14.0, Some("py"));
    let job_py = hl2.highlight("def main():");
    assert_eq!(job_py.text, "def main():");
}

#[test]
fn syntax_highlighter_multi_line() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let text = "line1\nline2\nline3";
    let job = hl.highlight(text);
    assert_eq!(job.text, text);
}

#[test]
fn syntax_highlighter_trailing_newline() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let text = "line1\nline2\n";
    let job = hl.highlight(text);
    assert_eq!(job.text, text);
}
