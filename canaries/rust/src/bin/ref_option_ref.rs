//! Canary.
#[rustfmt::skip]
/// Canary item.
fn take(opt: &Option<&u8>) -> bool { //~ clippy::ref_option_ref clippy::missing_const_for_fn clippy::trivially_copy_pass_by_ref
    opt.is_some()
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    sink(take(&None));
}
