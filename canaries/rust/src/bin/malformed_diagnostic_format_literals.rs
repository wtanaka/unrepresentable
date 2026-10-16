//! Canary.
#[rustfmt::skip]
#[diagnostic::on_unimplemented(message = "{")] //~ malformed_diagnostic_format_literals
/// Canary item.
trait Tr {} //~ dead_code

fn main() {}
