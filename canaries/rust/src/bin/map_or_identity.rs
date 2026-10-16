//! Canary.
/// Canary item.
fn pick(opt: Option<u8>) -> u8 {
    opt.map_or(5, |value| value) //~ clippy::map_or_identity
}

fn main() {
    assert_eq!(pick(None), 5, "pick");
}
