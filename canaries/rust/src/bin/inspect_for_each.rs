//! Canary.
#[rustfmt::skip]
fn main() {
    let data: Vec<u8> = Vec::new();
    data.iter().inspect(|item| assert!(**item > 0, "pos")).for_each(|item| assert!(*item > 0, "pos")); //~ clippy::inspect_for_each
}
