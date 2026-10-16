//! Canary.
#[rustfmt::skip]
fn main() {
    let data: [u8; 2] = [1, 2];
    let opt: Option<&[u8]> = Some(&data);
    if let Some(slice) = opt { //~ clippy::index_refutable_slice
        assert_eq!(slice[0], 1, "first"); //~ clippy::indexing_slicing
    }
}
