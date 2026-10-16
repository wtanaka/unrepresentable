//! Canary.
#[rustfmt::skip]
extern "C" { //~ missing_unsafe_on_extern
    fn go();
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(go);
}
