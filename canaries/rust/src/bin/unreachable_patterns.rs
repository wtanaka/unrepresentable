//! Canary.
fn main() {
    let value: Option<u8> = Some(1);
    let _: u8 = match value {
        _ => 1,
        None => 2, //~ unreachable_patterns
    };
}
