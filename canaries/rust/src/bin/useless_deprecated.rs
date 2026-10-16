//! Canary.
/// Canary item.
trait Tr {
    /// Canary item.
    fn function();
}

/// Canary item.
struct Item;

impl Tr for Item {
    #[deprecated] //~ useless_deprecated unused_attributes
    fn function() {}
}

fn main() {
    Item::function();
}
