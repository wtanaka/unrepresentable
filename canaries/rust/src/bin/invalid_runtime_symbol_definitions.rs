//! Canary.
#[rustfmt::skip]
#[unsafe(no_mangle)] //~ unsafe_code
/// Canary item.
extern "C" fn memcpy() {} //~ invalid_runtime_symbol_definitions clippy::missing_const_for_fn

fn main() {}
