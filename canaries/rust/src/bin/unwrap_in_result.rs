//! Canary.
#![deny(clippy::unwrap_in_result)]
use core::str::FromStr;
/// Canary item.
struct Holder;
impl FromStr for Holder {
    type Err = String;
    fn from_str(src: &str) -> Result<Self, String> {
        let _v = src.parse::<i32>().unwrap(); //~ clippy::unwrap_in_result clippy::unwrap_used
        Ok(Self)
    }
}
fn main() {
    let _v = Holder::from_str("1");
}
