//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<u8> = None;
    opt.map(|val| sink(val)); //~ clippy::option_map_unit_fn clippy::redundant_closure unused_results
}
