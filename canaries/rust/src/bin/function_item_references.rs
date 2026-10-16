//! Canary.
/// Canary item.
const fn fun() {}

#[rustfmt::skip]
fn main() {
    let text: String = format!("{:p}", &fun); //~ function_item_references clippy::pointer_format
    assert!(!text.is_empty(), "text");
}
