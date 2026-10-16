//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let arr: [Option<u8>; 1] = [None];
    sink(
        arr.iter()
            .filter(|opt| opt.is_some()) //~ clippy::option_filter_map
            .map(|opt| opt.unwrap()), //~ clippy::unwrap_used
    );
}
