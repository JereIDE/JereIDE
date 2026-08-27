use jereide_syntax::SyntaxHighlighter;
use jereide_settings::{syntax_number, syntax_string};

#[test]
fn syntax_highlighter_json() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("json"));
    let job = hl.highlight("{\"key\": 42, \"ok\": true}").clone();
    assert_eq!(job.text, "{\"key\": 42, \"ok\": true}");

    let mut saw_string = false;
    let mut saw_number = false;
    for s in &job.sections {
        if s.format.color == syntax_string() {
            saw_string = true;
        }
        if s.format.color == syntax_number() {
            saw_number = true;
        }
    }
    assert!(saw_string, "expected JSON strings to be highlighted");
    assert!(saw_number, "expected JSON numbers to be highlighted");
}

#[test]
fn syntax_highlighter_empty_text() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job = hl.highlight("").clone();
    assert_eq!(job.text, "");
}

#[test]
fn syntax_highlighter_plain_text() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job = hl.highlight("hello world").clone();
    assert_eq!(job.text, "hello world");
}

#[test]
fn syntax_highlighter_rust_keyword() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("rs"));
    let job = hl.highlight("fn main() {}").clone();
    assert_eq!(job.text, "fn main() {}");
}

#[test]
fn syntax_highlighter_cache_same_input() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let job1 = hl.highlight("hello").clone();
    let job2 = hl.highlight("hello").clone();
    assert_eq!(job1.text, job2.text);
}

#[test]
fn syntax_highlighter_cache_invalidated_on_change() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    hl.highlight("hello");
    let job = hl.highlight("world").clone();
    assert_eq!(job.text, "world");
}

#[test]
fn syntax_highlighter_html_tags() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("html"));
    let job = hl.highlight("<div class=\"note\">hi</div>").clone();
    assert_eq!(job.text, "<div class=\"note\">hi</div>");
    assert!(job.sections.len() > 1);
}

#[test]
fn syntax_highlighter_html_comment() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("html"));
    let job = hl.highlight("<!-- a comment --><p>x</p>").clone();
    assert_eq!(job.text, "<!-- a comment --><p>x</p>");
}

#[test]
fn syntax_highlighter_switching_extension() {
    let mut hl = SyntaxHighlighter::new(14.0, Some("rs"));
    let job_rs = hl.highlight("fn main() {}").clone();
    assert_eq!(job_rs.text, "fn main() {}");

    let mut hl2 = SyntaxHighlighter::new(14.0, Some("py"));
    let job_py = hl2.highlight("def main():").clone();
    assert_eq!(job_py.text, "def main():");
}

#[test]
fn syntax_highlighter_multi_line() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let text = "line1\nline2\nline3";
    let job = hl.highlight(text).clone();
    assert_eq!(job.text, text);
}

#[test]
fn syntax_highlighter_trailing_newline() {
    let mut hl = SyntaxHighlighter::new(14.0, None);
    let text = "line1\nline2\n";
    let job = hl.highlight(text).clone();
    assert_eq!(job.text, text);
}
