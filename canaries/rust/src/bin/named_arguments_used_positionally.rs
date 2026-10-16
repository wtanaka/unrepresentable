//! Canary.
fn main() {
    let number: u8 = 1;
    drop(format!("{}", a = number)); //~ named_arguments_used_positionally clippy::uninlined_format_args
}
