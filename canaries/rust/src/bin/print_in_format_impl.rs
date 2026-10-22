//! Canary.
use core::fmt;

#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl fmt::Display for Unit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("x"); //~ clippy::print_in_format_impl clippy::print_stdout
        write!(formatter, "s")
    }
}

#[rustfmt::skip]
fn main() {
    drop(Unit.to_string());
}
