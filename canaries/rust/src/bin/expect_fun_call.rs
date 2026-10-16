//! Canary.
#[rustfmt::skip]
fn main() {
    let value: Option<u8> = "1".parse().ok();
    let text = "value";
    let got = value.expect(&format!("missing {text}")); //~ clippy::expect_fun_call clippy::expect_used
    assert!(got == 1, "one");
}
