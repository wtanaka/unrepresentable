//! Canary.
#[rustfmt::skip]
unsafe extern { //~ unsafe_code missing_abi
    fn go();
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(go);
}
