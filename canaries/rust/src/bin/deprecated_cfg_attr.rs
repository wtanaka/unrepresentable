//! Canary.
#[cfg_attr(rustfmt, rustfmt_skip)] //~ clippy::deprecated_cfg_attr
/// Canary item.
const fn skipped() {}

fn main() {
    skipped();
}
