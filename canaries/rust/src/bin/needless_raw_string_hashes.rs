//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let text = r##"a"b"##; //~ clippy::needless_raw_string_hashes
    sink(text);
}
