//! Canary.
use std::io::stdin;

#[rustfmt::skip]
fn main() {
    let mut line = String::new();
    drop(stdin().read_line(&mut line));
    drop(line.parse::<i32>()); //~ clippy::read_line_without_trim
}
