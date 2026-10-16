//! Canary.
#[rustfmt::skip]
/// Canary item.
fn total(items: &[u8]) -> Option<u8> {
    items.iter().fold(Some(0), |acc, item| { //~ clippy::manual_try_fold
        acc.and_then(|sum| sum.checked_add(*item)) //~ clippy::return_and_then
    })
}

fn main() {
    assert_eq!(total(&[1, 2]), Some(3), "total");
}
