//! Canary.
fn main() {
    let items: Vec<u8> = Vec::new();
    core::mem::forget(items); //~ clippy::absolute_paths clippy::mem_forget
}
