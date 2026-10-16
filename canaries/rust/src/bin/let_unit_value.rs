//! Canary.
/// Canary item.
const fn nothing() {}

fn main() {
    let _unit = nothing(); //~ clippy::let_unit_value unit_bindings
}
