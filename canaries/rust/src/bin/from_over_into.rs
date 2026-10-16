//! Canary.
/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
impl Into<u8> for Wrapper { //~ clippy::from_over_into
    fn into(self) -> u8 {
        self.0
    }
}

fn main() {
    let raw: u8 = Wrapper(1).into();
    assert!(raw == 1, "one");
}
