//! Canary.
/** First line.  */ //~ clippy::doc_comment_double_space_linebreaks
/** Second line. */
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
