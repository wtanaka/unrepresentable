//! Canary.
use std::env;

#[rustfmt::skip]
fn main() {
    env::set_var("KEY", "val"); //~ deprecated_safe_2024
}
