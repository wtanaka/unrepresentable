//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let mut count: u8 = 0;
    if value == 0 {
        count = 1;
    } else if value == 1 { //~ clippy::else_if_without_else
        count = 2;
    }
    assert!(count > 0, "counted");
}
