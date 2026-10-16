//! Canary.
#[rustfmt::skip]
fn main() {
    let arr: [u8; 1] = [1];
    let bx: Box<[u8]> = Box::new(arr);
    let _iter = bx.into_iter(); //~ boxed_slice_into_iter
}
