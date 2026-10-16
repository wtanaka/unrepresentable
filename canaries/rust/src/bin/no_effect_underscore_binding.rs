//! Canary.
/// Canary item.
struct Thing;

fn main() {
    let _item = Thing; //~ clippy::no_effect_underscore_binding
}
