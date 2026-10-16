//! Canary.
use core::hint::black_box;

fn main() {
    let value: Option<u8> = Some(0);
    let Some(mut count) = &value else { return }; //~ rust_2024_incompatible_pat
    count = count.saturating_add(1);
    let _: u8 = black_box(count);
}
