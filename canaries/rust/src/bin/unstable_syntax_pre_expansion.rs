//! Canary.
#[cfg(false)]
fn gated() {
    let _ = try { 1 }; //~ unstable_syntax_pre_expansion
}

fn main() {}
