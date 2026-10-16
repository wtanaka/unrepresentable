//! Canary.
#[rustfmt::skip]
#[diagnostic::on_unimplemented(foo = "bar")] //~ malformed_diagnostic_attributes
/// Canary item.
trait Tr {} //~ dead_code

fn main() {}
