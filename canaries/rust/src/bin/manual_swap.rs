//! Canary.
#[rustfmt::skip]
/// Canary item.
fn swap(left: &mut u8, right: &mut u8) { //~ clippy::missing_const_for_fn
    let tmp = *left; //~ clippy::manual_swap
    *left = *right;
    *right = tmp;
}

fn main() {
    let mut left: u8 = 1;
    let mut right: u8 = 2;
    swap(&mut left, &mut right);
    assert_eq!(left, 2, "left");
}
