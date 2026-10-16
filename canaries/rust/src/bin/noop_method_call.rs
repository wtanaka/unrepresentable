//! Canary.
/// Canary item.
struct Item;

fn main() {
    let item = &Item;
    let _copy = item.clone(); //~ noop_method_call
}
