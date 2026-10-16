//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn show(value: &u8) -> bool { //~ clippy::trivially_copy_pass_by_ref
    *value > 0
}

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    assert!(show(&one), "positive");
}
