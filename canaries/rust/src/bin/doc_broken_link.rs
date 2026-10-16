//! Canary.
#[doc = "[bad link](https://"] //~ clippy::doc_broken_link
#[doc = "example.com/)."]
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
