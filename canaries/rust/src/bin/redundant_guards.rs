//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = black_box(None);
    match opt {
        Some(val) if val == 1 => {} //~ clippy::redundant_guards
        Some(_) | None => {}
    }
}
