//! Canary.
use core; //~ clippy::single_component_path_imports redundant_imports

#[rustfmt::skip]
fn main() {
    core::hint::spin_loop(); //~ clippy::absolute_paths
}
