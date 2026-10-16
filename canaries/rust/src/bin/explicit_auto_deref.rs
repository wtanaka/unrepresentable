//! Canary.
#[rustfmt::skip]
fn main() {
    let text = String::from("text");
    let view: &str = &*text; //~ clippy::explicit_auto_deref
    assert!(view.len() == 4, "four");
}
