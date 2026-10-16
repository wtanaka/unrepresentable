//! Canary.
#[rustfmt::skip]
/// Canary item.
fn takes(_arg: &dyn Drop) {} //~ dyn_drop

fn main() {
    let vec: Vec<u8> = Vec::new();
    takes(&vec);
}
