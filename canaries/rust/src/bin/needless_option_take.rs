//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<u8> = None;
    let res = opt.as_ref().take(); //~ clippy::needless_option_take
    sink(res);
}
