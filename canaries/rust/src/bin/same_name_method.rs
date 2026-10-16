//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Unit {
    /// Canary item.
    fn val(&self) {} //~ clippy::same_name_method clippy::unused_self clippy::missing_const_for_fn
}

#[rustfmt::skip]
/// Canary item.
trait Named {
    /// Canary item.
    fn val(&self);
}

#[rustfmt::skip]
impl Named for Unit {
    fn val(&self) {}
}

#[rustfmt::skip]
fn main() {
    Unit.val();
    Named::val(&Unit);
}
