//! Canary.
#[rustfmt::skip]
fn main() {
    let res: Result<u8, u8> = Ok(1);
    sink(res.map_or(None, Some)); //~ clippy::result_map_or_into_option
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
