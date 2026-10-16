//! Canary.
use core::mem;
use std::process;

fn main() {
    if true {
        process::abort()
    } else {
        // SAFETY: none.
        unsafe { mem::zeroed() } //~ never_type_fallback_flowing_into_unsafe unsafe_code
    }; //~ clippy::unnecessary_semicolon
}
