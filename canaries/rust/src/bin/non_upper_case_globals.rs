//! Canary.
use core::hint::black_box;

/// Canary item.
const my_const: u8 = 1; //~ non_upper_case_globals

fn main() {
    let _: u8 = black_box(my_const);
}
