//! Canary.
use core::hint::black_box;

fn main() {
    let µ: u8 = 1; //~ uncommon_codepoints non_ascii_idents clippy::disallowed_script_idents
    let _: u8 = black_box(µ);
}
