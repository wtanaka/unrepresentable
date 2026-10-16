//! Canary.
#[derive(Default, Eq, PartialEq)]
/// Canary item.
struct Unit;

fn main() {
    std::process::exit(i32::from(Unit::default() == Unit)); //~ clippy::default_constructed_unit_structs clippy::absolute_paths
}
