//! Canary.
#[rustfmt::skip]
fn main() {
    let mut data: Vec<u8> = Vec::new();
    for item in data.drain(..) { //~ clippy::iter_with_drain
        assert_eq!(item, 1, "one");
    }
}
