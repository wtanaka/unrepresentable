//! Canary.
/** Uses ['Documented'] for things. */ //~ clippy::doc_link_with_quotes
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
