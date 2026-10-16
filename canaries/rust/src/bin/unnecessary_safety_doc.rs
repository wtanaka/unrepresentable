//! Canary.
/// Doc.
///
/// # Safety
///
/// Nothing.
#[inline]
#[must_use]
pub const fn documented() -> bool { //~ clippy::unnecessary_safety_doc
    true
}
fn main() {
    let _v = documented();
}
