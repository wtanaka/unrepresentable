//! The integration test suite: one test target, so that helpers shared through
//! `tests/support/` are used by every test crate that compiles them.
#[macro_use]
mod support;

#[cfg(test)]
#[path = "suite/against.rs"]
mod against;

#[cfg(test)]
#[path = "suite/awkward.rs"]
mod awkward;

#[cfg(test)]
#[path = "suite/cli.rs"]
mod cli;

#[cfg(test)]
#[path = "suite/config_change.rs"]
mod config_change;

#[cfg(test)]
#[path = "suite/guards.rs"]
mod guards;

#[cfg(test)]
#[path = "suite/hardening.rs"]
mod hardening;

#[cfg(test)]
#[path = "suite/meta.rs"]
mod meta;

#[cfg(test)]
#[path = "suite/ratchet.rs"]
mod ratchet;

#[cfg(test)]
#[path = "suite/rules.rs"]
mod rules;

#[cfg(test)]
#[path = "suite/scope.rs"]
mod scope;

#[cfg(test)]
#[path = "suite/selftest.rs"]
mod selftest;

#[cfg(test)]
#[path = "suite/toml_reading.rs"]
mod toml_reading;

use clap as _;

use glob as _;

use proc_macro2 as _;

use serde_json as _;

use syn as _;

use toml as _;
