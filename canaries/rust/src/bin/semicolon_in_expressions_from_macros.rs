//! Canary.
/// Canary item.
macro_rules! mac {
    () => {
        7; //~ semicolon_in_expressions_from_macros
    };
}

fn main() {
    let selector: u8 = 1;
    let _: u8 = match selector {
        0 => 5,
        _ => mac!(),
    };
}
