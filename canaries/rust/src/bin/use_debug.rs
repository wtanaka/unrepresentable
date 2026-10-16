//! Canary.
use core::fmt::Write as _;
fn main() {
    let value: i32 = 1;
    let mut buf = String::new();
    let _v = write!(buf, "{value:?}"); //~ clippy::use_debug
}
