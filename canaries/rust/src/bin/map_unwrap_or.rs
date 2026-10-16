//! Canary.
/// Canary item.
fn pick(opt: Option<u8>) -> u8 {
    opt.map(|value| value.min(9)).unwrap_or(5) //~ clippy::map_unwrap_or
}

fn main() {
    assert_eq!(pick(None), 5, "pick");
}
