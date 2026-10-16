//! Canary.
use std::path::MAIN_SEPARATOR;

fn main() {
    let sep: &str = &MAIN_SEPARATOR.to_string(); //~ clippy::manual_main_separator_str
    assert!(!sep.is_empty(), "sep");
}
