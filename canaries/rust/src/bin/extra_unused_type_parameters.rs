//! Canary.
#[rustfmt::skip]
/// Canary item.
fn noop<Param>(value: u8) -> u8 { //~ clippy::extra_unused_type_parameters clippy::missing_const_for_fn
    value
}

fn main() {
    assert!(noop::<u8>(1) == 1, "one");
}
