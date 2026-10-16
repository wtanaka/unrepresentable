//! Canary.
#[rustfmt::skip]
/// Canary item.
fn unwrap_or_return(opt: Option<u8>) {
    let value = if let Some(value) = opt { //~ clippy::manual_let_else
        value
    } else {
        return;
    };
    assert!(value > 0, "value");
}

fn main() {
    unwrap_or_return(Some(1));
}
