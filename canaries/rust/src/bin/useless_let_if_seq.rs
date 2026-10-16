//! Canary.
fn main() {
    let src = String::new();
    let mut value = false; //~ clippy::useless_let_if_seq
    if src.is_empty() {
        value = true;
    }
    let _v = value.to_string();
}
