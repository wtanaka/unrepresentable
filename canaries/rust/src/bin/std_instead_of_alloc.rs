//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let shared = Rc::new(value);
    assert_eq!(*shared, value, "one");
}
