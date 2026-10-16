//! Canary.
/// Canary item.
const fn nothing() {}

fn main() {
    let _unit = nothing(); //~ unit_bindings clippy::let_unit_value
}
