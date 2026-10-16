//! Canary.
use std::process::exit;

fn main() {
    let opt: Option<u8> = Some(1);
    let res = opt.and_then(Some); //~ clippy::bind_instead_of_map
    exit(i32::from(res.is_none()));
}
