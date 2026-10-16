//! Canary.
/// Canary item.
struct Holder<'life>(&'life u8);

#[rustfmt::skip]
/// Canary item.
const fn read(arg: Holder) -> u8 { //~ elided_lifetimes_in_paths clippy::needless_pass_by_value
    *arg.0
}

fn main() {
    let num: u8 = 1;
    assert_eq!(read(Holder(&num)), 1, "value");
}
