//! Canary.
use std::path::Path;

#[rustfmt::skip]
fn main() {
    let path = Path::new("a");
    let text = format!("{path:?}"); //~ clippy::unnecessary_debug_formatting
    assert!(!text.is_empty(), "text");
}
