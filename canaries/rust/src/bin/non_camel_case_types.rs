//! Canary.
use core::hint::black_box;

/// Canary item.
type my_type = u8; //~ non_camel_case_types

fn main() {
    let _: my_type = black_box(1);
}
