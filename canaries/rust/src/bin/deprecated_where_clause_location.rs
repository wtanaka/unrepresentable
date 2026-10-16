//! Canary.
/// Canary item.
trait Tr {
    /// Canary item.
    type Assoc<Ty>
    where
        Ty: Copy;
}

/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Tr for Unit {
    type Assoc<Ty> where Ty: Copy = Ty; //~ deprecated_where_clause_location
}

fn main() {
    let unit: Unit = Unit;
    assert!(matches!(unit, Unit), "unit");
    let num: <Unit as Tr>::Assoc<u8> = 1;
    assert_eq!(num, 1, "value");
}
