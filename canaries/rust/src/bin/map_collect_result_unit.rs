//! Canary.
#[rustfmt::skip]
/// Canary item.
fn check(item: u8) -> Result<(), u8> { //~ clippy::missing_const_for_fn
    if item > 1 {
        Err(item)
    } else {
        Ok(())
    }
}

/// Canary item.
fn all(items: &[u8]) -> Result<(), u8> {
    items.iter().map(|item| check(*item)).collect() //~ clippy::map_collect_result_unit
}

fn main() {
    assert!(all(&[1]).is_ok(), "all");
}
