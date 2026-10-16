//! Canary.
use core::fmt::Write as _;
fn main() {
    let mut buf = String::new();
    let _v = write!(buf, "a\n"); //~ clippy::write_with_newline
}
