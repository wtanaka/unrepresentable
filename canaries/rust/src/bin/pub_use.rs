//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    pub use core::cmp::max; //~ clippy::pub_use unreachable_pub
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    sink(inner::max(one, one));
}
