//! Canary.
use std::path::Path;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let path = Path::new("a.md");
    sink(path.ends_with(".md")); //~ clippy::path_ends_with_ext
}
