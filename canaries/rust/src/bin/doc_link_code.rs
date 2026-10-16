//! Canary.
/** See [`first`](https://example.com)`second`. */ //~ clippy::doc_link_code
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
