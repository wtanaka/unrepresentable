//! Canary.

const fn helper() {} //~ clippy::missing_docs_in_private_items

/// Entry point.
fn main() {
    helper();
}
