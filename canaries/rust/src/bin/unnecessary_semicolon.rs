//! Canary.
fn main() {
    let src = Some(true);
    match src {
        Some(_) | None => {}
    }; //~ clippy::unnecessary_semicolon
}
