//! Canary.
#[doc = ""] //~ clippy::empty_docs
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
