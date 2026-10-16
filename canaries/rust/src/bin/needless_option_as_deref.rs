//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let mut opt: Option<&mut u8> = None;
    let res = opt.as_deref_mut(); //~ clippy::needless_option_as_deref
    sink(res);
}
