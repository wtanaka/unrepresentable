//! Canary.
#[ignore] //~ clippy::ignore_without_reason
/// Canary item.
const fn skipped() {}

fn main() {
    skipped();
}
