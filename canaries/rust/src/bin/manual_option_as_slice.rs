//! Canary.
use core::slice;

fn main() {
    let opt: Option<u8> = Some(1);
    let one = opt.as_ref().map_or(&[][..], slice::from_ref); //~ clippy::manual_option_as_slice
    assert_eq!(one.len(), 1, "len");
}
