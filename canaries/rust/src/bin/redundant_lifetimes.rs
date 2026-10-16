//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
const fn function<'long>(text: &'long str) -> &'static str //~ redundant_lifetimes
where
    'long: 'static,
{
    text
}

#[rustfmt::skip]
fn main() {
    let _: &str = black_box(function("a"));
}
