//! Canary.
#[rustfmt::skip]
fn main() {
    while true { //~ while_true clippy::never_loop
        break;
    }
}
