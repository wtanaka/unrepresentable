//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn consume(_value: u8) {}

#[rustfmt::skip]
fn main() {
    let mut data: [u8; 2] = [2, 1];
    data.sort_by_key(|each| { //~ clippy::unit_return_expecting_ord
        consume(*each);
    });
    assert!(data.len() == 2, "len");
}
