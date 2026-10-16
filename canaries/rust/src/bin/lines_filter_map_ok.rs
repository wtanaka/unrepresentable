//! Canary.
use std::io::BufRead as _;

fn main() {
    let data: &[u8] = b"";
    let num = data.lines().filter_map(Result::ok).count(); //~ clippy::lines_filter_map_ok
    assert_eq!(num, 0, "empty");
}
