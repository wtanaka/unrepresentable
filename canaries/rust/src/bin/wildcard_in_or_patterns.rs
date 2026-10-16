//! Canary.
fn main() {
    let src = true;
    let _v = match src {
        true | _ => true, //~ clippy::wildcard_in_or_patterns
    };
}
