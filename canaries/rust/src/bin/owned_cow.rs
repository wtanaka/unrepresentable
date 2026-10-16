//! Canary.
use std::borrow::Cow; //~ clippy::std_instead_of_alloc

/// Canary item.
fn takes(_val: Cow<'static, String>) {} //~ clippy::owned_cow
fn main() {
    takes(Cow::Owned(String::new()));
}
