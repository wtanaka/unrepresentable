//! Canary.
#[cfg_attr(feature = "cargo-clippy", allow(dead_code))] //~ clippy::deprecated_clippy_cfg_attr unexpected_cfgs
/// Canary item.
const fn skipped() {}

fn main() {
    skipped();
}
