//! Canary.
#[rustfmt::skip]
fn main() {
    let value: Option<u8> = Some(1);
    let text = match value { //~ clippy::single_match_else
        Some(1) => "one",
        _ => {
            let other = String::new();
            assert!(other.is_empty(), "empty");
            "other"
        }
    };
    assert!(!text.is_empty(), "text");
}
