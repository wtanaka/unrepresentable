//! Canary.
/// Canary item.
const VALUE: u32 = 0x12_345_67; //~ clippy::unusual_byte_groupings
fn main() {
    let _v = VALUE.to_string();
}
