//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
enum Void {}

#[rustfmt::skip]
unsafe extern "C" { //~ unsafe_code
    static VOID: Void; //~ uninhabited_static
}

#[rustfmt::skip]
fn main() {
    let _: *const Void = black_box(&raw const VOID);
}
