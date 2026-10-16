//! Canary.
/** 1. First item.
lazy continuation. */ //~ clippy::doc_lazy_continuation
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
