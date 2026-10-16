//! Canary.
use std::process::exit;

fn main() {
    let text = String::from("x");
    let rf = &text;
    let rr: &String = &*rf; //~ clippy::borrow_deref_ref
    exit(i32::from(rr.is_empty()));
}
