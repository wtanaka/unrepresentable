//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(res: Result<u8, u8>) -> u8 {
    match res {
        Ok(value) => value,
        Err(_) => panic!("bad"), //~ clippy::match_wild_err_arm clippy::panic
    }
}

fn main() {
    assert_eq!(pick(Ok(1)), 1, "pick");
}
