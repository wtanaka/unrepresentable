//! Canary.
fn main() {
    let src = vec![true, false]; //~ clippy::useless_vec
    let _v = src.contains(&true);
    // Canary: 200 bytes, above the old `too-large-for-stack` of 64.
    let big = vec![[u8::MAX; 100], [u8::MIN; 100]]; //~ clippy::useless_vec
    let _w = big.contains(&[u8::MAX; 100]);
}
