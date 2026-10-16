//! Canary.
use std::io::{stderr, Write as _};

#[rustfmt::skip]
fn main() {
    writeln!(stderr(), "message").unwrap(); //~ clippy::explicit_write clippy::unwrap_used
}
