//! Canary.
use std::env::args;

fn main() {
    let flag = args().count() == 1;
    assert_eq!(flag, true, "canary"); //~ clippy::bool_assert_comparison
}
