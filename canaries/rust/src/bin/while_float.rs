//! Canary.
fn main() {
    let mut value: f32 = 0.0;
    while value < 1.0 { //~ clippy::while_float
        value = value.next_up();
    }
}
