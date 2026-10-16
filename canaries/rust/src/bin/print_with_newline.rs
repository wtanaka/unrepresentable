//! Canary.
#[rustfmt::skip]
fn main() {
    print!("a\n"); //~ clippy::print_with_newline clippy::print_stdout
}
