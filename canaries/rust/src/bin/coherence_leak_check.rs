//! Canary.
#[rustfmt::skip]
/// Canary item.
trait Tr {} //~ dead_code

#[rustfmt::skip]
impl Tr for for<'arg> fn(&'arg u8) {} //~ single_use_lifetimes
#[rustfmt::skip]
impl Tr for fn(&'static u8) {} //~ coherence_leak_check

fn main() {}
