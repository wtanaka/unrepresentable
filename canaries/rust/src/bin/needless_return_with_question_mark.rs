//! Canary.
/// Canary item. The lint fires only on a `return Err(x)?;` that is not the last
/// statement of the fn; at the tail, clippy reports `needless_return` and
/// `try_err` instead.
fn check(flag: Option<u8>) -> Result<u8, u8> {
    if flag.is_none() {
        return Err(1)?; //~ clippy::needless_return_with_question_mark clippy::try_err
    }
    Ok(2)
}

fn main() {
    let outcome: Result<u8, u8> = check(Some(1));
    assert!(outcome.is_ok(), "ok");
}
