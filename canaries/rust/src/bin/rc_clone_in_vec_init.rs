//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let list: Vec<Rc<u8>> = vec![Rc::new(1); 2]; //~ clippy::rc_clone_in_vec_init
    sink(list);
}
