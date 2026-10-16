//! Canary.
fn main() {
    let _v = Some(true).map(u8::from); //~ clippy::unnecessary_map_on_constructor
}
