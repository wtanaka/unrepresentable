//! Canary.
#[rustfmt::skip]
/// Canary item.
fn never() -> ! { //~ clippy::missing_const_for_fn
    loop {} //~ clippy::empty_loop
}

fn main() {
    let flag = false;
    if flag {
        never();
    }
}
