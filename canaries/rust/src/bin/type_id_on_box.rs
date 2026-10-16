//! Canary.
use core::any::{Any, TypeId};

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    let boxed: Box<dyn Any> = Box::new(one);
    let identity = boxed.type_id(); //~ clippy::type_id_on_box
    assert!(identity != TypeId::of::<u8>(), "box");
}
