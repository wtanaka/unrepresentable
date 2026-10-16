//! Canary.
use std::process::exit;

fn main() {
    let mut aa = String::from("y");
    let bb = String::from("x");
    let before = aa.len();
    aa = bb.clone(); //~ clippy::assigning_clones
    exit(i32::from(aa.len() == before || bb.is_empty()));
}
