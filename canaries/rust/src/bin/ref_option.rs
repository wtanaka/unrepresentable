//! Canary.
#[rustfmt::skip]
/// Canary item.
fn take(opt: &Option<String>) -> usize { //~ clippy::ref_option
    opt.as_ref().map_or(0, String::len)
}

#[rustfmt::skip]
fn main() {
    sink(take(&None));
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
