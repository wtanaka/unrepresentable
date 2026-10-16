//! Canary.
/** Documented. */ //~ clippy::empty_line_after_doc_comments

#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
