//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let text: String = items.iter().map(|item| format!("{item}")).collect(); //~ clippy::format_collect
    assert!(text.len() == 2, "two");
}
