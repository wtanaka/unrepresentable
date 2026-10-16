# Running in CI

This page describes how to run `unrepresentable` on every pull request in CI.

## Install and verify

Install from crates.io at an exact version:

```sh
cargo install unrepresentable --locked --version =<exact version>
```

`--locked` makes cargo use the dependency versions in the published crate's
`Cargo.lock`. Write `=<exact version>`; cargo ignores a pre-release version
otherwise. Upgrade only in a separate pull request (see
[Stability](stability.md#upgrade-advice)).

Alternatively, use the installer script. Each release carries an asset named
`unrepresentable.sh`, which holds the version and the sha256 of each release
binary. To use the script:

1. Download `unrepresentable.sh` from the release you want to pin and commit it
   to your repository, for example as `ci/unrepresentable.sh`.
2. Keep the script at `ci/unrepresentable.sh`, which the tool treats as a
   protected path: a change to that path exits 3 (see
   [Change control](change-control.md)). At another path, list the file under
   `judge` in the configuration.
3. The CI job calls `sh ci/unrepresentable.sh` after a full checkout (see
   "Checkout" below).

The script downloads the binary for the platform (Linux x86_64 or aarch64, macOS
x86_64 or aarch64, Windows x86_64), compares the sha256 of the download with the
value written into the script, and exits 2 on a mismatch without running the
binary. The script then runs, in this order, `self-test`,
`check --against "origin/${GITHUB_BASE_REF:-main}"` and `canaries`. The script
reads no setting from the environment except `GITHUB_BASE_REF`, and needs `curl`
or `wget`, `git`, and `sha256sum` or `shasum`. The tool's exit codes pass
through. Without the script, run the commands below.

## Commands

Run these in order from the repository root.

1. `unrepresentable self-test` checks the binary against its built-in canaries
   and needs no repository. A failure means the binary must not be trusted.
2. `unrepresentable check --against <base> .` runs the checks. `<base>` is a ref
   that the CI configuration chose, such as `origin/main`; `--against REF` means
   `REF...HEAD`. `--range RANGE` names the range directly. Do not rely on
   `default-range` in CI: the configuration under review supplies it (see
   [`default-range` is not a trust anchor](configuration.md#default-range-is-not-a-trust-anchor)).
   With `--against`, the range may be empty, so a push to the default branch
   passes.
3. `unrepresentable canaries .` runs the canaries of the repository (see
   [Lint coverage and canaries](lint-coverage.md#self-test-and-canaries)).
   `self-test` and `canaries` take no range: `--against` and `--range` are usage
   errors (exit 2) for them.

## Checkout

`check` refuses a shallow clone. Check out with full history (for GitHub
Actions, `fetch-depth: 0`) so that the base ref exists locally, for example
`origin/main`.

## Toolchain prerequisites

`check` enforces `lint-coverage` (see
[Lint coverage and canaries](lint-coverage.md)) and needs:

- a pinned toolchain: `rust-toolchain.toml` with an exact `channel`, for example
  `channel = "1.98.1"`; a missing or unpinned toolchain is a `lint-coverage`
  finding;
- clippy installed for that toolchain, at the exact version the tool's
  strictest-knob table was written for; an unknown clippy version exits 2.

## Exit codes in a job

| Exit | Job action                                                                                                                                                                                                                                                                            |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | Pass.                                                                                                                                                                                                                                                                                 |
| 1    | Fail. At least one finding.                                                                                                                                                                                                                                                           |
| 2    | Fail. The tool could not check the code (usage error, unparsable input, git or I/O failure, bad configuration, no range, shallow clone).                                                                                                                                              |
| 3    | The code rules pass and a human must approve the change, because the change touches the checks themselves (configuration, ledgers, baseline, protected paths). The job must not report the change as approved. Print the `approval:` lines and require a human decision before merge. |

Exit 1 takes precedence over exit 3. The installer script passes the tool's exit
code through, so a job that runs the script fails on 1, 2 and 3 alike. The full
list of causes for each code is in the [Command line](cli.md#exit-codes)
reference.
