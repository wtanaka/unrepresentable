//! Canary.
#[rustfmt::skip]
/// Canary item.
fn first(opt: Option<u8>) -> Option<String> {
    if opt.is_none() { //~ clippy::question_mark
        return None;
    }
    Some(String::from("a"))
}

#[rustfmt::skip]
fn main() {
    drop(first(None));
}
