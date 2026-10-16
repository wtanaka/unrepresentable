//! Canary.
#[rustfmt::skip]
fn main() {
    let data: Vec<u8> = Vec::new();
    for item in data.iter().next() { //~ clippy::iter_next_loop for_loops_over_fallibles
        assert_eq!(*item, 0, "zero");
    }
}
