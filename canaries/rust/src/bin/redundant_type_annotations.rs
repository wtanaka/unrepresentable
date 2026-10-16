//! Canary.
#[rustfmt::skip]
fn main() {
    let text: String = String::new(); //~ clippy::redundant_type_annotations
    drop(text);
}
