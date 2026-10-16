//! Canary.
#[rustfmt::skip]
/// This documentation paragraph is intentionally very long without any blank line so that it exceeds //~ clippy::too_long_first_doc_paragraph clippy::doc_markdown
/// the two hundred character threshold used by the lint to decide that a summary line is missing here
/// and then it keeps going for a little while longer just to be certain that the limit is passed.
///
/// More text.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Thing;

#[rustfmt::skip]
fn main() {
    let thing = Thing;
    assert!(size_of_val(&thing) == 0, "zero");
}
