//! Canary.
#[rustfmt::skip]
fn main() {
    let _ptr: *const u8 = String::from("a").as_ptr(); //~ dangling_pointers_from_temporaries
}
