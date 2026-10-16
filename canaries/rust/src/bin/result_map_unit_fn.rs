//! Canary.
#[rustfmt::skip]
/// Canary item.
fn show(_val: u8) { //~ clippy::missing_const_for_fn
}

#[rustfmt::skip]
fn main() {
    let res: Result<u8, u8> = Ok(1);
    res.map(show); //~ clippy::result_map_unit_fn unused_must_use
}
