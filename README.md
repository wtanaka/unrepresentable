# unrepresentable

A Rust linter that rejects code whose types and signatures allow invalid states.
`unrepresentable` is for repos where AI coding agents write much of the Rust.

**Status: alpha.** Rules, config, ledger and baseline formats and exit behaviour
may change in any release without backwards compatibility, and a new always-on
rule can make a passing CI run fail after an upgrade. Pinning an exact version
(`--version =<exact version>`) avoids that until the upgrade is chosen.

## Philosophy

A value that is invalid for the domain must not be constructible. Constraints
are expressed in types (newtypes with validating constructors, enums with one
variant per legal case, non-empty collections), not in runtime checks or tests
that run afterwards. For example, a price is a `Cents` newtype and not a bare
`u64`, and two flags that cannot both be set are one enum. This tool checks that
code follows that rule wherever the syntax makes a violation visible. The tool
complements tests, clippy and other lints and does not replace them.

`fn charge(cents: u64, refund: bool)` compiles and passes clippy. This tool
rejects that signature, because `u64` and `bool` do not constrain the values the
function accepts. A `Cents` newtype and a `Charge`/`Refund` enum do.

## What the rules reject

`unrepresentable` applies a fixed set of rules, listed in
[docs/rules.md](https://github.com/wtanaka/unrepresentable/blob/main/docs/rules.md).
Code that passes has, in every scope:

- No panics or aborts (`unwrap`, `panic!`, `assert!` outside tests).
- No swallowed errors or discarded values (`unwrap_or`, `ok()`, `let _ =`).
- Exhaustive `match` on your own enums, with no catch-all arm.
- Domain values in named types, not bare integers, strings or `bool` flags.
- No unchecked arithmetic, numeric casts, floats or `HashMap` iteration order.
- No file, environment, process, network or clock access outside designated I/O
  crates.
- No lint suppression except recorded, justified exceptions.

The tool analyzes source syntax only. Inferred types, procedural-macro output
and the contents of dependencies are not analyzed, so violations there are not
reported. See
[Known limits](https://github.com/wtanaka/unrepresentable/blob/main/docs/limits.md).
The rules are designed for code that agents write, so they favor strictness over
authoring ergonomics.

## Scope

- Clippy is required, pinned and at its strictest settings (see `lint-coverage`
  in the
  [rules](https://github.com/wtanaka/unrepresentable/blob/main/docs/rules.md)).
- Changes to the tool's own configuration and records (ledgers, baseline,
  canaries, protected paths) are controlled, so the author of a change cannot
  weaken the rules that evaluate the change.

## Quickstart

```sh
cargo install unrepresentable --locked --version =0.0.1-alpha.1
unrepresentable self-test
```

`--locked` makes cargo use the dependency versions in the published crate's
`Cargo.lock`. Always write `--version =<exact version>`; cargo ignores a
pre-release otherwise.

Adopt on an existing codebase:

```sh
unrepresentable init .                              # writes config, ledger, exemptions
unrepresentable check --against origin/main .       # checks the change
```

`init` writes `.unrepresentable/config`, `ledger.md` and `canary-exemptions.md`;
review the config before the first run (see
[Configuration](https://github.com/wtanaka/unrepresentable/blob/main/docs/configuration.md)).
Repository-level findings (`manifest-lints`, `lint-coverage`) cannot be
baselined and fail the first `check` until fixed.

`check --against REF` (or `check --range RANGE`) names the change to check.
`default-range` is a convenience for local use, not a trust anchor: CI must
always pass `--range` or `--against` with a base that CI chose itself.
Subcommands, ranges and output are in the
[command-line reference](https://github.com/wtanaka/unrepresentable/blob/main/docs/cli.md);
a CI job is described in
[Running in CI](https://github.com/wtanaka/unrepresentable/blob/main/docs/ci.md).

| Exit | Meaning                                                                                                                                      |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | Clean                                                                                                                                        |
| 1    | At least one finding                                                                                                                         |
| 2    | The tool could not check the code (usage error, unparsable input, git or I/O failure, bad config, no range). CI must treat this as a failure |
| 3    | No findings, but the change touches something a human must approve                                                                           |

## Turning rules off

The rules cannot be turned off: they have no off switch, warn level,
per-directory opt-out or in-source suppression, and the rules apply to all code,
including tests. If you want a configurable linter, use clippy alone. An unknown
config key is a parse error. `.unrepresentable/config` only says which paths are
code, which crates may do I/O, and which specific items are exempt from a rule,
each with a written reason. The two exception mechanisms are recorded in files a
human approves: `#[expect(lint, reason = "..")]` for a compiler or clippy lint
(the check fails without a matching row in `.unrepresentable/ledger.md`), and
the baseline of existing violations, which can only shrink.

## Documentation

### For adopters

| Page                                                                                                     | Contents                                                             |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| [Command line](https://github.com/wtanaka/unrepresentable/blob/main/docs/cli.md)                         | Modes, the change range, output, exit codes.                         |
| [Running in CI](https://github.com/wtanaka/unrepresentable/blob/main/docs/ci.md)                         | Pinned install, the commands a job runs, exit codes in a job.        |
| [Configuration](https://github.com/wtanaka/unrepresentable/blob/main/docs/configuration.md)              | `init`, config keys and globs, `default-range`, fingerprints.        |
| [Rules](https://github.com/wtanaka/unrepresentable/blob/main/docs/rules.md)                              | Every rule id and what it rejects.                                   |
| [Lint coverage and canaries](https://github.com/wtanaka/unrepresentable/blob/main/docs/lint-coverage.md) | `lint-coverage`, the omission ledger, `self-test`, `canaries`.       |
| [Change control](https://github.com/wtanaka/unrepresentable/blob/main/docs/change-control.md)            | `check --ratchet`, judge and protected paths, boundaries.            |
| [Known limits](https://github.com/wtanaka/unrepresentable/blob/main/docs/limits.md)                      | Macro bodies, and what the tool does not analyze or does not report. |
| [Stability](https://github.com/wtanaka/unrepresentable/blob/main/docs/stability.md)                      | What alpha means, what may change, upgrade advice.                   |

### For maintainers

| Page                                                                                 | Contents                                         |
| ------------------------------------------------------------------------------------ | ------------------------------------------------ |
| [Contributing](https://github.com/wtanaka/unrepresentable/blob/main/CONTRIBUTING.md) | Repository layout, build and test, review rules. |
| [Security](https://github.com/wtanaka/unrepresentable/blob/main/SECURITY.md)         | Reporting a vulnerability.                       |

## License

Licensed under either of
[Apache License, Version 2.0](https://github.com/wtanaka/unrepresentable/blob/main/LICENSE-APACHE)
or
[MIT license](https://github.com/wtanaka/unrepresentable/blob/main/LICENSE-MIT)
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
