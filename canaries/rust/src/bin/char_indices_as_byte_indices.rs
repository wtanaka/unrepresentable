//! Canary.
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let text = "str";
    for (idx, _) in text.chars().enumerate() { //~ clippy::never_loop
        exit(i32::from(text[..idx].is_empty())); //~ clippy::char_indices_as_byte_indices clippy::string_slice
    }
}
