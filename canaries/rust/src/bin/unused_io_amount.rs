//! Canary.
use std::io::{Result, Write as _};
/// Canary item.
fn run() -> Result<()> {
    let mut buf: Vec<u8> = Vec::new();
    buf.write(b"x")?; //~ clippy::unused_io_amount unused_results
    Ok(())
}
fn main() {
    let _v = run();
}
