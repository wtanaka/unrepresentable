//! Canary.
/// First line //~ clippy::doc_comment_double_space_linebreaks clippy::doc_markdown  
/// Second line.
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
