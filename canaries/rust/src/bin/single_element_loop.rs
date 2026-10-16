//! Canary.
#[rustfmt::skip]
fn main() {
    let item: u8 = 1;
    for each in [item] { //~ clippy::single_element_loop
        assert_eq!(each, item, "one");
    }
}
