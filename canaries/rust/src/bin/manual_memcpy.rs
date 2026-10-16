//! Canary.
#[rustfmt::skip]
/// Canary item.
fn copy(dst: &mut [u8], src: &[u8]) {
    for idx in 0..src.len() { //~ clippy::manual_memcpy
        dst[idx] = src[idx]; //~ clippy::indexing_slicing
    }
}

fn main() {
    let mut dst = [0; 1];
    copy(&mut dst, &[1]);
    assert_eq!(dst.len(), 1, "len");
}
