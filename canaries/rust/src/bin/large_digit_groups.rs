//! Canary.
fn main() {
    let val: u64 = 6186491_8973511; //~ clippy::large_digit_groups
    assert!(val > 1, "big");
}
