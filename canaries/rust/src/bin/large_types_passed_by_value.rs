//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn total(arr: [u8; 300]) -> usize { //~ clippy::large_types_passed_by_value
    arr.len()
}

fn main() {
    assert_eq!(total([0; 300]), 300, "len");
}
