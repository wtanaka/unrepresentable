//! Canary.
use core::hint::spin_loop;

/// Canary item.
fn never() -> ! {
    loop {
        spin_loop();
    }
}

#[rustfmt::skip]
fn main() {
    let ready = true;
    let value = (ready || never()) || ready; //~ clippy::diverging_sub_expression
    assert!(value, "ready");
}
