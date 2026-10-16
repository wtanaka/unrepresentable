//! Canary.
#[doc = "- [link]: description"] //~ clippy::doc_nested_refdefs
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
