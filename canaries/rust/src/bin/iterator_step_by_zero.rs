//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let num = data.iter().step_by(0).count(); //~ clippy::iterator_step_by_zero
    assert_eq!(num, 0, "empty");
}
