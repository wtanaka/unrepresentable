//! Canary.
/// Canary item.
enum Kind {
    /// Canary item.
    Var,
}

/// Canary item.
trait Tr {
    /// Canary item.
    type Var;
    /// Canary item.
    fn make() -> Self::Var;
}

#[rustfmt::skip]
impl Tr for Kind {
    type Var = u8;
    fn make() -> Self::Var { //~ ambiguous_associated_items
        0
    }
}

fn main() {
    let kind: Kind = Kind::Var;
    assert!(matches!(kind, Kind::Var), "variant");
    let num: u8 = <Kind as Tr>::make();
    assert_eq!(num, 0, "value");
}
