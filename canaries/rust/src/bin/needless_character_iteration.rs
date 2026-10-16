//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let ascii = "abc".chars().all(|chr| chr.is_ascii()); //~ clippy::needless_character_iteration
    sink(ascii);
}
