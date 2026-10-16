//! Canary.
/// Canary item.
trait Tr {
    /// Canary item.
    fn function(mut value: u8); //~ patterns_in_fns_without_body
}

/// Canary item.
struct Item;

impl Tr for Item {
    fn function(_value: u8) {}
}

fn main() {
    Item::function(1);
}
