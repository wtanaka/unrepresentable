//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    drop(format!("{:p}", &val)); //~ clippy::pointer_format
}
