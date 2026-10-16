//! Canary.
fn main() {
    let _v = Some(true).unwrap_or(false); //~ clippy::unnecessary_literal_unwrap
}
