//! Canary.
#[rustfmt::skip]
fn main() {
    let result: Result<u8, u8> = Err(1);
    let error = result.err().expect("an error"); //~ clippy::err_expect clippy::expect_used
    assert!(error == 1, "one");
}
