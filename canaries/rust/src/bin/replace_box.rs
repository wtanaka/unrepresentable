//! Canary.
#[rustfmt::skip]
/// Canary item.
fn replace(boxed: &mut Box<u8>) {
    *boxed = Box::new(2); //~ clippy::replace_box
}

#[rustfmt::skip]
fn main() {
    let mut boxed: Box<u8> = Box::new(1);
    replace(&mut boxed);
    drop(boxed);
}
