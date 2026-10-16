//! Canary.
use std::fs::File;
use std::io::{Read as _, Result};
/// Canary item.
fn run() -> Result<()> {
    let mut file = File::open("a")?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?; //~ clippy::verbose_file_reads unused_results
    Ok(())
}
fn main() {
    let _v = run();
}
