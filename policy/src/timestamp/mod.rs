//! UTC timestamps as seconds since the Unix epoch, from the one format the
//! GitHub API uses: `YYYY-MM-DDTHH:MM:SSZ`.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::parse_utc;
