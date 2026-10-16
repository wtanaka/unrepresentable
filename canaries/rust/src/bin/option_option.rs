//! Canary.
/// Canary item.
const fn takes(_val: Option<Option<u8>>) {} //~ clippy::option_option
fn main() {
    takes(None);
}
