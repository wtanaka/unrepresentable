//! Canary.
#[rustfmt::skip]
/// Canary item.
fn show(res: Result<u8, u8>) {
    if let Some(value) = res.ok() { //~ clippy::match_result_ok
        assert!(value > 0, "value");
    }
}

fn main() {
    show(Ok(1));
}
