//! Canary.
#[non_exhaustive]
/// Canary item.
enum Kind {
    /// Canary item.
    First,
    /// Canary item.
    Second,
}
/// Canary item.
const fn make(flag: &str) -> Kind {
    if flag.is_empty() {
        Kind::First
    } else {
        Kind::Second
    }
}
fn main() {
    let _v = match make("") {
        Kind::First {} => true, //~ clippy::unneeded_struct_pattern
        Kind::Second => false,
    };
}
