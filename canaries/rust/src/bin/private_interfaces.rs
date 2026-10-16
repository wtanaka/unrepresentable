//! Canary.
/// Canary item.
struct Item;

#[inline]
/// Canary item.
pub const fn takes(_item: Item) {} //~ private_interfaces

fn main() {
    takes(Item);
}
