//! Canary.
/** A documented value */ //~ clippy::doc_paragraphs_missing_punctuation
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
