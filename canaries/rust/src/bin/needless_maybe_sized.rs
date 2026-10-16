//! Canary.
/// Canary item.
const fn func<Typ>(_val: &Typ)
where
    Typ: Sized + ?Sized, //~ clippy::needless_maybe_sized
{
}
fn main() {
    let one: u8 = 1;
    func(&one);
}
