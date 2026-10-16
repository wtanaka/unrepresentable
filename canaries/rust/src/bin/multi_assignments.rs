//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let mut aaa = (); //~ unused_assignments
    let mut bbb: u8 = 0; //~ unused_assignments
    aaa = bbb = 1; //~ clippy::multi_assignments
    sink((aaa, bbb));
}
