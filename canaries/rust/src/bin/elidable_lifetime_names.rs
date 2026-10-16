//! Canary.
/// Canary item.
struct Holder<'item>(&'item u8);

#[rustfmt::skip]
impl<'item> Holder<'item> { //~ clippy::elidable_lifetime_names single_use_lifetimes
    /// Canary item.
    fn get(&self) -> u8 { //~ clippy::missing_const_for_fn
        *self.0
    }
}

fn main() {
    let value: u8 = 1;
    assert!(Holder(&value).get() == 1, "one");
}
