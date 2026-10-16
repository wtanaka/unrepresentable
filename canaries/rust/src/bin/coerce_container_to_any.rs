//! Canary.
use core::any::Any;
use std::process::exit;

fn main() {
    let boxed: Box<dyn Any> = Box::new(String::new());
    let any: &dyn Any = &boxed; //~ clippy::coerce_container_to_any
    exit(i32::from(any.is::<String>()));
}
