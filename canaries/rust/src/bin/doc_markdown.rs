//! Canary.
/** Wraps a FooBar value. */ //~ clippy::doc_markdown
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
