//! Canary.
use core::hint::black_box;

fn main() {
    let _: &str = black_box("‮"); //~ text_direction_codepoint_in_literal clippy::non_ascii_literal
}
