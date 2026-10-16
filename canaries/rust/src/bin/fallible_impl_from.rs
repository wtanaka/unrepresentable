//! Canary.
/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
impl From<u32> for Wrapper { //~ clippy::fallible_impl_from
    fn from(value: u32) -> Self {
        Self(u8::try_from(value).unwrap()) //~ clippy::unwrap_used
    }
}

fn main() {
    assert!(Wrapper::from(1).0 == 1, "one");
}
