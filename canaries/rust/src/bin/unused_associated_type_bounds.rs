//! Canary.
/// Canary item.
trait Tr {
    /// Canary item.
    type Assoc
    where
        Self: Sized;
}

/// Canary item.
struct Item;

impl Tr for Item {
    type Assoc = u8;
}

/// Canary item.
const fn function(_object: &dyn Tr<Assoc = u8>) {} //~ unused_associated_type_bounds

fn main() {
    function(&Item);
}
