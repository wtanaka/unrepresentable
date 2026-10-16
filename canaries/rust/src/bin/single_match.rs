//! Canary.
#[rustfmt::skip]
fn main() {
    let value: Option<u8> = Some(1);
    match value { //~ clippy::single_match
        Some(inner) => assert!(inner > 0, "positive"),
        _ => {}
    }
}
