//! Canary.
/** A value[^1]. */ //~ clippy::doc_suspicious_footnotes
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
