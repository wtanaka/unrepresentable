//! Canary.
#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = Some(1);
    for _item in opt {} //~ for_loops_over_fallibles
}
