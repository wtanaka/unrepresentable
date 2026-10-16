//! Canary.
#[rustfmt::skip]
fn main() {
    let text = String::from("a");
    let other = text.clone(); //~ clippy::redundant_clone
    drop(other);
}
