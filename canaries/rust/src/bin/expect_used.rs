//! Canary.
#[rustfmt::skip]
fn main() {
    let value: Option<u8> = "1".parse().ok();
    let got = value.expect("a value"); //~ clippy::expect_used
    assert!(got == 1, "one");
}
