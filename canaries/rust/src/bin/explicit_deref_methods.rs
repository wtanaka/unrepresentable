//! Canary.
use core::ops::Deref as _;

#[rustfmt::skip]
fn main() {
    let text = String::from("text");
    let view: &str = text.deref(); //~ clippy::explicit_deref_methods
    assert!(view.len() == 4, "four");
}
