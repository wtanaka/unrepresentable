//! Canary.
fn main() {
    let _v = concat!("a").len(); //~ clippy::useless_concat
}
