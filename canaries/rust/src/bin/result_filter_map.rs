//! Canary.
#[rustfmt::skip]
fn main() {
    let list: Vec<&str> = Vec::new();
    sink(list.into_iter().map(str::parse::<u8>).filter(Result::is_ok).map(Result::unwrap).count()); //~ clippy::result_filter_map
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
