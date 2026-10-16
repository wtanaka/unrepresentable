# Development

This page is for maintainers of this repository. It covers building and testing
the repository, its dependencies, how its CI checks the repository with the tool
itself, and the project status. Contribution rules are in
[CONTRIBUTING.md](../../CONTRIBUTING.md).

## Repository layout

- `src/`: the tool, the `unrepresentable` crate.
- `tests/`: integration tests, fixtures, and the rule canaries.
- `policy/`: the `unrepresentable-policy` crate, the merge-policy tooling.
- `ci/` and `.github/`: the check scripts and the workflows.
- `canaries/`: the clippy canaries, one file per lint.
- `docs/`: pages for projects that use the tool. `docs/maintainers/` holds the
  pages for people who change this repository.

## Building and testing

CI calls cargo only through [`ci/cargo.sh`](../../ci/cargo.sh) so that every
call uses the same flags. [`ci/checks.sh`](../../ci/checks.sh) is the list of
checks that CI runs. Both print a short usage with `--help`, and their header
comments are the reference. [ci/README.md](../../ci/README.md) says what each
script is for.

The merge-policy crate is a separate workspace member with its source in
`policy/`. The wrapper takes `-p unrepresentable-policy` to select it. Some
files in `canaries/` carry `#[rustfmt::skip]` so that formatting does not move a
trailing `//~` mark; for that reason `cargo fmt --all` is not used.

`unrepresentable` needs a change range for `check` (`--range`, `--against`, or a
config `default-range` that selects some change; see [Command line](../cli.md)).
A bare `run -- check .` therefore exits 2 on the default branch, where
`main...HEAD` is empty. The `owntree` check always passes a range: the last
commit by default, and `BASE_SHA...HEAD` in the head-checks of a pull request.

Tests are described in [tests/README.md](../../tests/README.md).

## Dependencies

`unrepresentable` reads Rust syntax, TOML, JSON, path globs and its own command
line, and the policy crate also reads and writes base64. Each of these is done
by a third-party crate, not by code in this repository. A parser written here
has to be correct for every manifest and every API answer it meets, and the
earlier hand-written readers were not: the TOML reader refused valid TOML, and
the manifest reader in the policy crate skipped any line it did not understand.
A widely used parser has had more review and fuzzing than a private one. The
cost is more third-party code that runs in CI, so every dependency is
exact-pinned, takes no default features (except `proc-macro2`, whose default
`proc-macro` feature stays on next to `span-locations`), and is checked by
`cargo-deny`.

| Crate         | Version                                                     | Used by      | Why                                                                                                                                                                                                                                                        |
| ------------- | ----------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `proc-macro2` | `=1.0.107`                                                  | main         | token spans (`span-locations`)                                                                                                                                                                                                                             |
| `clap`        | `=4.6.7` (`cargo`, `std`, `error-context`, `usage`, `help`) | main         | command line: the subcommands `check`, `init`, `self-test` and `canaries`, and the rule that `--against` and `--range` exclude each other. Builder API only: no derive macros, no color, no suggestions. Pulls in `clap_builder`, `clap_lex` and `anstyle` |
| `syn`         | `=2.0.119`                                                  | main         | Rust syntax tree                                                                                                                                                                                                                                           |
| `toml`        | `=1.1.6` (`parse`, `std`)                                   | main, policy | TOML 1.1: manifests, `.unrepresentable/config`, `clippy.toml`, cargo config, `Cargo.lock`                                                                                                                                                                  |
| `serde_json`  | `=1.0.151` (`std`)                                          | main, policy | JSON: cargo's `--message-format=json`, GitHub API answers, verdict data. Untyped values only: no derive macros, no `preserve_order`, so objects are sorted maps. Numbers are normalised, not kept as written (see below)                                   |
| `glob`        | `=0.3.4`                                                    | main, policy | path globs in the config; CODEOWNERS patterns                                                                                                                                                                                                              |
| `base64`      | `=0.23.1` (`alloc`)                                         | policy       | check run data, artifact fields, the fetch credential                                                                                                                                                                                                      |

Every direct dependency has `=x.y.z` in the manifest, `default-features = false`
(except `proc-macro2`, see above), the smallest feature list that builds, and a
version that was at least two weeks old when it was chosen. `Cargo.lock` is
committed and every cargo call is `--locked`. `deny.toml` allows only the MIT,
Apache-2.0 and Unicode-3.0 licenses, one version of each crate, no wildcard
requirements and crates.io as the only source. Advisories (vulnerabilities,
yanked and unmaintained crates) are checked on pushes and by the policy sweep.
The lock lists every third-party package, including optional dependencies that
Cargo records in the lock without building them.

Each crate is used behind a module that already existed (`src/toml`, `src/json`,
`src/glob`, `policy/src/toml`, `policy/src/json`, `policy/src/bytes`,
`policy/src/codeowners`), and those modules return the same types as before.
Ledgers (markdown tables), `tools.lock` and the workflow files are not standard
formats and are still read by code in this repository.

JSON numbers are normalised by `serde_json`: an integer that fits `i64` or `u64`
keeps its digits, and anything else becomes an `f64` and is written back in that
form (`1.0e2` as `100.0`, `-0` as `-0.0`, `18446744073709551616` as
`1.8446744073709552e+19`). The `arbitrary_precision` feature does not keep the
text either (`1.0e2` came back as `1.0e+2`, `-0` as `0`), so it is not enabled.
Nothing depends on the bytes of a re-written untrusted number: cargo messages
are read for strings and counts, and the policy crate compares and displays API
values after parsing, never forwarding their text.

## Self-policing CI

A change must not be able to alter the checker that evaluates it, so CI runs a
pinned version of the tool whose hash is verified.
[`ci/unrepresentable.sh`](../../ci/unrepresentable.sh) is the consumer entry
point: it installs the pinned binary, checks its sha256, then runs `self-test`,
`check --against` and `canaries` (see [Running in CI](../ci.md) and
[ci/README.md](../../ci/README.md)). The merge policy is implemented as policy
tooling under `policy/` and `.github/`: a GitHub App owns the required `policy`
check, pull requests are squash-merged at the evaluated head SHA, and exit 3
requires human approval. One workflow, `.github/workflows/policy.yml`, holds
both the pull request jobs and the checks for pushes to `main`. See
[policy/README.md](../../policy/README.md) for the merge policy, its
scratch-repository test plan and its required repository settings.

## Status

The scanner, rules, ledgers, baseline, `check --against`, canaries, `self-test`,
the integration tests in `tests/`, the unit tests in `src/` and the unit tests
of the policy crate (`policy/`) run in this repository, and both crates pass
their own rules with an empty baseline. The policy tooling, workflows and
release installer have not been exercised on a real GitHub repository. Test and
canary counts are not given here; the `test` check reports the current ones.
