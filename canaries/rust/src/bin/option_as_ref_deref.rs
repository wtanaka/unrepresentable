//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<String> = None;
    sink(opt.as_ref().map(String::as_str)); //~ clippy::option_as_ref_deref
}
