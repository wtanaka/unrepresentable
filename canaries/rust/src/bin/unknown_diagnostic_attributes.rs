//! Canary.
use core::hint::black_box;

#[diagnostic::unknown_attr] //~ unknown_diagnostic_attributes
/// Canary item.
struct Item;

fn main() {
    let _: Item = black_box(Item);
}
