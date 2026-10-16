//! Canary.
fn main() {
    let num: u8 = 1;
    const fn helper() {} //~ clippy::items_after_statements
    helper();
    assert_eq!(num, 1, "one");
}
