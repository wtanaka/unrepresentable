//! Canary.
fn main() {
    let _ = async {}; //~ clippy::let_underscore_future let_underscore_drop
}
