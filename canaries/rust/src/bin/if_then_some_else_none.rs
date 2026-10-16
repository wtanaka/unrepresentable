//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let count = args().count();
    let res: Option<usize> = if count > 1 { Some(count) } else { None }; //~ clippy::if_then_some_else_none
    exit(i32::from(res.is_some()));
}
