//! Canary.
fn main() {
    let _v = format!("{:5}", format_args!("{}", "a")); //~ clippy::unused_format_specs
}
