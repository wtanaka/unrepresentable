//! Canary.
#[rustfmt::skip]
/// Canary item.
fn find_three(items: &[u8]) -> Option<&u8> {
    for item in items { //~ clippy::manual_find
        if *item == 3 {
            return Some(item);
        }
    }
    None
}

fn main() {
    assert_eq!(find_three(&[3]), Some(&3), "find");
}
