//! Canary.
/// Canary item.
fn conv(res: Result<u8, u8>) -> Result<u8, ()> {
    res.map_err(|_| ()) //~ clippy::map_err_ignore
}

fn main() {
    assert!(conv(Ok(1)).is_ok(), "conv");
}
