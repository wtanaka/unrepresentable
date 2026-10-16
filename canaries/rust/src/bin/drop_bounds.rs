//! Canary.
/// Canary item.
fn takes<Ty>(_arg: Ty)
where
    Ty: Drop, //~ drop_bounds
{
}

fn main() {
    let vec: Vec<u8> = Vec::new();
    takes(vec);
}
