//! Canary.
use core::fmt::Write; //~ clippy::unused_trait_names
fn main() {
    let mut buf = String::new();
    let _v = buf.write_str("x");
}
