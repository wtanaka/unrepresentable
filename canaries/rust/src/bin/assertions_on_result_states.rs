//! Canary.
use core::num::ParseIntError;

fn main() {
    let res: Result<u8, ParseIntError> = "1".parse();
    assert!(res.is_ok()); //~ clippy::assertions_on_result_states clippy::missing_assert_message
}
