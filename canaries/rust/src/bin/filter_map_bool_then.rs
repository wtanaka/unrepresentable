//! Canary.
#[rustfmt::skip]
/// Canary item.
fn double(value: u8) -> u8 { //~ clippy::missing_const_for_fn
    value.saturating_mul(2)
}

#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let total = items.iter().filter_map(|item| (*item > 1).then(|| double(*item))); //~ clippy::filter_map_bool_then
    assert!(total.count() == 1, "one");
}
