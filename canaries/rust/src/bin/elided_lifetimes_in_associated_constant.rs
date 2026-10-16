//! Canary.
/// Canary item.
struct Holder<'life>(&'life u8);

#[rustfmt::skip]
impl<'life> Holder<'life> { //~ clippy::elidable_lifetime_names single_use_lifetimes
    /// Canary item.
    const VAL: &u8 = &1; //~ elided_lifetimes_in_associated_constant
}

fn main() {
    let val: &u8 = Holder::VAL;
    assert_eq!(*val, 1, "value");
    assert_eq!(*Holder(val).0, 1, "inner");
}
