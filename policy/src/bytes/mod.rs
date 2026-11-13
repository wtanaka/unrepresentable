//! Raw bytes, and base64 over them (the `base64` crate, standard alphabet).

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{decode, encode, Bytes};
