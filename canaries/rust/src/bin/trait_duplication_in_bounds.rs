//! Canary.
#[rustfmt::skip]
/// Canary item.
fn check<T>(value: &T) -> T
where
    T: Clone + Clone, //~ clippy::trait_duplication_in_bounds
{
    value.clone()
}

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    assert!(check(&one) > 0, "positive");
}
