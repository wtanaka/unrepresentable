//! Canary.
#[derive(Clone, Copy, Debug)]
#[rustfmt::skip]
/// Canary item.
pub struct Config { //~ clippy::manual_non_exhaustive
    /// Canary item.
    _private: (),
    /// Canary item.
    pub field: u8, //~ clippy::partial_pub_fields
}

fn main() {
    let cfg = Config {
        _private: (),
        field: 1,
    };
    assert_eq!(cfg.field, 1, "field");
}
