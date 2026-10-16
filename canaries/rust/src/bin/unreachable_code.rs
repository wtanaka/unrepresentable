//! Canary.
/// Canary item.
const fn helper() {}

fn main() {
    return;
    helper(); //~ unreachable_code
}
