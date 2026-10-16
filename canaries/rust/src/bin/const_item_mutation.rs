//! Canary.
/// Canary item.
const VEC: Vec<u8> = Vec::new();

#[rustfmt::skip]
fn main() {
    VEC.push(1); //~ const_item_mutation
}
