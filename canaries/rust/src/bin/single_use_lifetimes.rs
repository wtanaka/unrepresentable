//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
struct Holder(usize);

#[rustfmt::skip]
/// Canary item.
const fn get<'life>(item: &'life Holder) -> usize { //~ single_use_lifetimes clippy::needless_lifetimes
    item.0
}

#[rustfmt::skip]
fn main() {
    let _: usize = black_box(get(&Holder(1)));
}
