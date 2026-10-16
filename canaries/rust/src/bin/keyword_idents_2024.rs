//! Canary.
#[rustfmt::skip]
fn main() {
    let gen: u8 = 1; //~ keyword_idents_2024
    assert_eq!(gen, 1, "value"); //~ keyword_idents_2024
}
