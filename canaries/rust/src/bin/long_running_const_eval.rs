//! Canary.
#[rustfmt::skip]
/// Canary item.
const COUNT: u32 = {
    let mut idx: u32 = 0;
    while idx < 3_000_000 { //~ long_running_const_eval
        idx += 1;
    }
    idx
};

fn main() {
    assert!(COUNT > 0, "count");
}
