//! Canary.
#[rustfmt::skip]
#[diagnostic::on_unimplemented(message = "bar")] //~ misplaced_diagnostic_attributes
/// Canary item.
struct Unit; //~ dead_code

fn main() {}
