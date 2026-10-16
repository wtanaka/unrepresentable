//! Canary.
use core::fmt;

#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("x"); //~ clippy::print_in_format_impl clippy::print_stdout
        write!(f, "s")
    }
}

#[rustfmt::skip]
fn main() {
    drop(Unit.to_string());
}
