//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn func<Fun: Copy>(_val: Fun) //~ clippy::multiple_bound_locations clippy::inline_trait_bounds
where
    Fun: Clone, //~ clippy::type_repetition_in_bounds
{
}
#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    func(one);
}
