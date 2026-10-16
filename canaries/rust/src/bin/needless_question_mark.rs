//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

/// Canary item.
fn pass(opt: Option<u8>) -> Option<u8> {
    Some(opt?) //~ clippy::needless_question_mark
}
fn main() {
    sink(pass(None));
}
