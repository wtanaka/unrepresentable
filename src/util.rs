//! Small helpers for optional slices.

use crate::types::{Answer, Count, Holds as _, Name};
use core::mem;
use std::ffi::OsStr;
use std::path::Path;

/// Removes the item at `index`, keeping the order of the rest.
pub(crate) fn remove_index<T>(items: &mut Vec<T>, index: Count) {
    let kept: Vec<T> = mem::take(items)
        .into_iter()
        .enumerate()
        .filter(|(position, _)| *position != index.get())
        .map(|(_, item)| item)
        .collect();
    *items = kept;
}

/// The items of an optional slice, or an empty slice.
pub(crate) const fn or_empty<T>(items: Option<&[T]>) -> &[T] {
    match items {
        Some(found) => found,
        None => &[],
    }
}

/// The items after the first `count`; empty when there are fewer.
pub(crate) fn skip_items<T>(items: &[T], count: Count) -> &[T] {
    or_empty(items.get(count.get()..))
}

/// Whether the file name has a `rs` extension.
pub(crate) fn is_rust_file(name: &Name) -> Answer {
    (Path::new(name.as_str()).extension() == Some(OsStr::new("rs"))).answer()
}
