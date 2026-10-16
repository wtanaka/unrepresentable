//! Canary.
fn main() {
    let value = String::new();
    let _v = format!("{}!", &value); //~ clippy::useless_borrows_in_formatting
}
