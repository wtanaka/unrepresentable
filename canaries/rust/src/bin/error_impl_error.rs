//! Canary.
use core::error;
use core::fmt::{Display, Formatter, Result};

#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
/// Canary item.
pub struct Error; //~ clippy::error_impl_error clippy::exhaustive_structs

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("error")
    }
}

impl error::Error for Error {}

fn main() {
    assert!(Error.to_string().len() == 5, "five");
}
