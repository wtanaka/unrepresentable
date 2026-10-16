# ci/

This page is for maintainers of this repository. It lists the scripts in this
directory. Each script has a header comment that is the reference for its
options and names; the scripts that take options print a short usage with
`--help`.

## Scripts

- [`cargo.sh`](cargo.sh): the cargo entry point of CI. The header comment lists
  the flags it adds and the calls it refuses.
- [`checks.sh`](checks.sh): the one list of checks (format, clippy, tests, the
  tool on its own tree, the publish check, cargo-deny). CI and local runs call
  the same names. The header comment describes each check, the `owntree`
  exit-code mapping and the offline and network requirements.
- [`version.sh`](version.sh): prints the `[package]` version of a `Cargo.toml`
  read from standard input and validates it. `checks.sh package` uses it.
- [`unrepresentable.sh`](unrepresentable.sh): the consumer installer template. A
  release fills in the version and the sha256 of every release binary and
  publishes the result as the release asset `unrepresentable.sh`. Run directly
  from this repository, the template exits with an error because its hash
  placeholders are not filled in.

## Where the checks are called

The cargo calls that `unrepresentable` itself makes go through `process::cargo`
in `src/process.rs`, the only place that spawns cargo.
