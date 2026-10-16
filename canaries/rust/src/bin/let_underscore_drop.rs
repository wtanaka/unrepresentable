//! Canary.
/// Canary item.
fn make() -> String {
    String::from("a")
}

#[rustfmt::skip]
fn main() {
    let _: String = make(); //~ let_underscore_drop clippy::redundant_type_annotations
}
