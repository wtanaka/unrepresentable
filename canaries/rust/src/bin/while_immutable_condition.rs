//! Canary.
/// Canary item.
const fn make() -> bool {
    true
}
fn main() {
    let flag = make();
    while flag { //~ clippy::while_immutable_condition
        if make() {
            break;
        }
    }
}
