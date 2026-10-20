# ci/

This page is for maintainers of this repository. It lists the scripts in this
directory and the workflows that call them. Each script has a header comment
that is the reference for its options and names; the scripts that take options
print a short usage with `--help`.

## Scripts

- [`cargo.sh`](cargo.sh): the cargo entry point of the workflows. The header
  comment lists the flags it adds and the calls it refuses.
- [`checks.sh`](checks.sh): the one list of checks (format, clippy, tests, the
  tool on its own tree, the publish check, workflow audit, cargo-deny). CI and
  local runs call the same names. The header comment describes each check, the
  `owntree` exit-code mapping and the offline and network requirements.
- [`version.sh`](version.sh): prints the `[package]` version of a `Cargo.toml`
  read from standard input and validates it. `checks.sh package` and the tag
  check of `release.yml` use it, and `policy/stage-sandbox.sh` copies it beside
  `checks.sh`.
- [`unrepresentable.sh`](unrepresentable.sh): the consumer installer template.
  The release workflow fills in the version and the sha256 of every release
  binary (`unrepresentable-policy make-installer`) and publishes the result as
  the release asset `unrepresentable.sh`. Run directly from this repository, the
  template exits with an error because its hash placeholders are not filled in.
  How a project installs and runs the asset is described in
  [docs/ci.md](../docs/ci.md).

The scripts of `policy/` (`sandbox.sh`, `offline.sh`, `stage-sandbox.sh`,
`take-file.sh`) are described in [policy/README.md](../policy/README.md) and in
their header comments.

## Where the checks are called

The head checks in `.github/workflows/policy.yml` use the copies of `cargo.sh`
and `checks.sh` from the base checkout, so a pull request cannot change the
flags or the list of checks that apply to it. Tests in
`policy/src/workflow_checks.rs` pin this: that no workflow line runs a bare
`cargo`, that every check name used in a workflow exists in `checks.sh`, and
that the push jobs and the head-checks job run the expected checks.

The cargo calls that `unrepresentable` itself makes go through `process::cargo`
in `src/process.rs`, the only place that spawns cargo.

## Workflow layout

`.github/workflows/policy.yml` holds both the pull request jobs and the checks
for pushes to `main`. Every job is gated by `github.event_name`.

- Pull requests (`pull_request_target`): `build-base`, then `classify`,
  `self-lint`, `head-checks`, `build-head`, `differential`,
  `differential-hidden`, `mutants`, then `policy-evaluate`. Whatever touches
  head code or head data with cargo in reach runs as an unprivileged user with
  no network (`policy/sandbox.sh`, `offline.sh`, `stage-sandbox.sh`). See
  [policy/README.md](../policy/README.md).
- Pushes to `main` and manual runs: `skip-check`, then in parallel `fmt`,
  `clippy-main`, `clippy-policy`, `deny` (`deny` then `deny-advisories`),
  `test`, `policytest`, `canaries`, `self-checks` (`package`, `selftest`,
  `owntree` in one job) and `audit-workflows`.

`skip-check` runs `unrepresentable-policy push-skip` for the pushed commit. It
compares the git tree of that commit with the tree of the head of each pull
request that contains it, and skips the other jobs only when a head has the same
tree and the App's `policy` check run for that head completed with success. In
every other case the checks run: a missing or failed check run, a different
tree, no pull request, an API error or a failed job. Manual runs never skip. A
skipped push also skips the advisory check; the hourly sweep covers that case.

The pinned zizmor and cargo-deny are installed from `policy/tools.lock` with
`unrepresentable-policy install-tool` (see
[policy/README.md](../policy/README.md)). No workflow writes to `GITHUB_PATH` or
`GITHUB_ENV`, because zizmor reports that as `github-env`; the pinned tool is
put on `PATH` inline for the one command that needs it, and the toolchain
channel travels as a validated step output passed to later steps with `env:`.

## Release

A tag is `v` plus the `Cargo.toml` version, for example `v0.0.2-alpha.1`. When
the version has a pre-release suffix, `release.yml` creates the GitHub release
with `--prerelease` and says in the notes that it is an alpha; the `.crate` file
and its unpack directory then carry the suffix too
(`unrepresentable-0.0.2-alpha.1`).

`release.yml` refuses a tag unless the tagged commit passed the push checks.
Every job checks out `github.sha` (not the tag name) and asserts that `HEAD` is
that commit. Its `verify` job, after the tag, version and default-branch checks,
builds the policy tool from the tagged tree (trusted, because the commit is on
the default branch) and runs
`unrepresentable-policy release-gate "$GITHUB_SHA"`. The gate passes when the
nine push jobs (`fmt`, `clippy-main`, `clippy-policy`, `deny`, `test`,
`policytest`, `canaries`, `self-checks`, `audit-workflows`) each have a check
run from the Actions app, in the `policy-pr` push run for that commit, that
succeeded; or when those jobs were skipped, `skip-check` succeeded and
`push-skip` would answer yes for the commit. In every other case, including a
pending job or an API error, the gate exits nonzero with a one-line reason. The
run must be a push to the default branch itself (`head_branch`), from the
workflow file at its exact path (or at `@refs/heads/<default branch>`); a run of
another branch that holds the same commit does not count.

The other workflows are in their own files because their triggers differ:
`release.yml` (tag push), `policy-review.yml` (`workflow_run` after
`review-signal.yml`), `review-signal.yml` (`pull_request_review`),
`policy-sweep.yml` (schedule: the approval sweep and the advisories check) and
`settings-audit.yml` (schedule).
