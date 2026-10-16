//! Canary.
use core::fmt::Write as _;
fn main() {
    let mut buf = String::new();
    let _v = writeln!(buf, ""); //~ clippy::writeln_empty_string
}
