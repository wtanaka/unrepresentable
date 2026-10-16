//! Canary.
/// Canary item.
fn need(opt: Option<u8>) -> Result<u8, u8> {
    opt.map_or(Err(1), Ok) //~ clippy::manual_ok_or
}

fn main() {
    assert_eq!(need(Some(1)), Ok(1), "need");
}
