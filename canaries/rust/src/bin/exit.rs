//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn quit() -> ! {
    exit(0) //~ clippy::exit
}

fn main() {
    let flag = false;
    if flag {
        quit();
    }
}
