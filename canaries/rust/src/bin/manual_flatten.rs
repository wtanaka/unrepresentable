//! Canary.
#[rustfmt::skip]
/// Canary item.
fn gather(items: &[Option<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items { //~ clippy::manual_flatten
        if let Some(value) = item {
            out.push(*value);
        }
    }
    out
}

fn main() {
    assert_eq!(gather(&[Some(1)]), vec![1], "gather");
}
