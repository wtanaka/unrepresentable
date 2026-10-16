//! Canary.
/** - First item
      continued here. */ //~ clippy::doc_overindented_list_items
#[rustfmt::skip]
struct Documented(u8);

fn main() {
    assert!(Documented(1).0 > 0, "positive");
}
