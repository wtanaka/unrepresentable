//! Canary.
#[rustfmt::skip]
fn main() {
    println!("{}", "a"); //~ clippy::print_literal clippy::print_stdout
}
