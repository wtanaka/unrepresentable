//! Canary.
#[derive(Default)]
/// Canary item.
struct Config {
    /// Canary item.
    size: u8,
}

#[rustfmt::skip]
fn main() {
    let mut config = Config::default();
    config.size = 1; //~ clippy::field_reassign_with_default
    assert!(config.size == 1, "one");
}
