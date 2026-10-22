//! Canary.
use core::fmt;

#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl fmt::Display for Unit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_string()) //~ clippy::recursive_format_impl
    }
}

#[rustfmt::skip]
fn main() {
    drop(Unit.to_string());
}
