//! Canary.
fn main() {
    let src = vec![true, false]; //~ clippy::useless_vec
    let _v = src.contains(&true);
}
