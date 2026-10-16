//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<u8> = None;
    sink(opt.map_or(None, Some)); //~ clippy::option_map_or_none
}
