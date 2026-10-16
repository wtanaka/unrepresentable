//! Canary.
use core::borrow::Borrow;
use core::hash::{Hash, Hasher};

/// Canary item.
struct Key(String);

#[rustfmt::skip]
impl Hash for Key { //~ clippy::impl_hash_borrow_with_str_and_bytes
    fn hash<State>(&self, state: &mut State)
    where
        State: Hasher,
    {
        self.0.hash(state);
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<[u8]> for Key {
    fn borrow(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

fn main() {
    let key = Key(String::new());
    let text: &str = key.borrow();
    let bytes: &[u8] = key.borrow();
    assert_eq!(text.len(), bytes.len(), "same");
}
