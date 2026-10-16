//! Canary.
#[rustfmt::skip]
fn main() {
    loop { //~ clippy::never_loop
        break;
    }
}
