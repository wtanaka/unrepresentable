//! Canary.
#[derive(PartialEq)]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Eq for Unit {
    fn assert_receiver_is_total_eq(&self) {} //~ internal_eq_trait_method_impls
}

fn main() {
    let one = Unit;
    let two = Unit;
    assert!(one == two, "eq");
}
