//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
const fn read(val: &mut u8) -> u8 { //~ clippy::needless_pass_by_ref_mut
    *val
}
#[rustfmt::skip]
fn main() {
    let mut val: u8 = 1;
    sink(read(&mut val));
}
