//! Canary.
#[rustfmt::skip]
/// Canary item.
fn noop<'unused>() {} //~ clippy::extra_unused_lifetimes unused_lifetimes clippy::missing_const_for_fn

fn main() {
    noop();
}
