//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink("\033[0m"); //~ clippy::octal_escapes
}
