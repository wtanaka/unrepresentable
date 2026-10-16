//! Canary.
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Unit {
    /// Canary item.
    const fn late<'arg>(self, val: &'arg u8) -> &'arg u8 { //~ clippy::needless_lifetimes clippy::unused_self
        val
    }
}

#[rustfmt::skip]
fn main() {
    let num: u8 = 1;
    assert_eq!(Unit.late::<'static>(&num), &1, "value"); //~ late_bound_lifetime_arguments
}
