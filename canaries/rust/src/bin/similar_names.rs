//! Canary.
#[rustfmt::skip]
fn main() {
    let apple: u8 = 1;
    let appla: u8 = 2; //~ clippy::similar_names
    assert!(apple < appla, "ordered");
}
