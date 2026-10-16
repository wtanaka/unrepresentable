//! Canary.
#[rustfmt::skip]
fn main() {
    let mut flag = true;
    let was = if flag { //~ clippy::manual_take
        flag = false;
        true
    } else {
        false
    };
    assert!(was, "was");
    assert!(!flag, "flag");
}
