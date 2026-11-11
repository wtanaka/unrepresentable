//! Canary.
fn main() {
    let value = return; //~ clippy::diverging_sub_expression
    value.clone(); //~ unreachable_code method_call_on_diverging_infer_var clippy::clone_on_copy unused_must_use
}
