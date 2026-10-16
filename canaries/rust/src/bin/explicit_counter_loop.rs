//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let mut count: usize = 0;
    for item in &items { //~ clippy::explicit_counter_loop
        assert!(*item as usize > count, "positive"); //~ clippy::as_conversions
        count += 1; //~ clippy::arithmetic_side_effects
    }
}
