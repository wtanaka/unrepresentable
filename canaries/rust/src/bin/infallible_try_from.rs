//! Canary.
use core::convert::Infallible;

/// Canary item.
struct Wrap(u8);

#[rustfmt::skip]
impl TryFrom<u8> for Wrap { //~ clippy::infallible_try_from
    type Error = Infallible;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        Ok(Self(val))
    }
}

fn main() {
    let res: Result<Wrap, Infallible> = Wrap::try_from(1);
    assert!(res.is_ok_and(|wrap| wrap.0 == 1), "ok");
}
