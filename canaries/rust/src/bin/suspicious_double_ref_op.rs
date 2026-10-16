//! Canary.
/// Canary item.
struct Item;

fn main() {
    let inner = &Item;
    let outer = &inner;
    let _copy = outer.clone(); //~ suspicious_double_ref_op
}
