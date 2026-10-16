#!/bin/sh
# The one list of checks for this repository. CI (the push jobs and the
# head-checks job of .github/workflows/policy.yml) and local runs all call it.
#
# usage: sh ci/checks.sh NAME [EXTRA_CARGO_ARGS...]
#        sh ci/checks.sh -h | --help
#
#   fmt              rustfmt, both crates (--check)
#   clippy           clippy for both crates (clippy-main then clippy-policy)
#   clippy-main      clippy for the main crate, all targets, warnings denied
#   clippy-policy    clippy for the policy crate, all targets, warnings denied
#   test             the main crate's tests
#   policytest       the policy crate's tests
#   selftest         the tool's `self-test`
#   canaries         the tool's `canaries`
#   owntree          the tool on this repository, over an explicit change range:
#                    `owntree [--range RANGE]`. The tool exits 2 when it is not
#                    given a range and the config's `default-range` selects no
#                    changes (a push to the default branch), so the range is
#                    always passed. Without --range it is `HEAD~1..HEAD` (the
#                    last commit; `HEAD` alone for a root commit). A pull
#                    request's head-checks pass `BASE_SHA...HEAD`. RANGE is
#                    `HEAD`, `HEAD~1..HEAD`, or two 40-digit commit ids (or one
#                    and `HEAD`) joined by `..` or `...`; anything else exits 2.
#                    Needs a full clone (the tool refuses a shallow one). The
#                    tool's exit 1 (findings) and 2 (could not decide) fail
#                    the check; its exit 3 (needs human approval) passes it,
#                    because approval is decided by the merge policy: a push
#                    to the default branch was approved when it was merged, and
#                    a pull request needs approval before it merges. Any other
#                    status (cargo itself failing) fails with that status.
#                    The first commit of a repository is the whole tree, so the
#                    default range `HEAD` contains every judge path and every
#                    code path, the `judge-code-pairing` rule reports a
#                    finding, and owntree fails once, on the first push.
#   package          the publish check. It replaces `publish --dry-run`, whose
#                    verification build uses the dev profile and creates a
#                    `debug` directory. It reads the version with ci/version.sh,
#                    packages the crate, unpacks the .crate in a scratch
#                    directory, requires the files the tool needs at runtime to
#                    be in it, builds it in release mode and runs its
#                    `self-test` (`--allow-dirty` is passed to `cargo package`
#                    alone, and only when the tree is dirty). The published
#                    file set is the explicit `include` list of the root
#                    Cargo.toml. `tests/` is not published, because its helpers
#                    read the `[workspace.lints*]` tables of the repository's
#                    root manifest, which `cargo package` rewrites into
#                    `[lints]` tables; the tests run on the repository, and the
#                    packaged crate is checked by its build and `self-test`.
#   audit-workflows  zizmor over .github (zizmor must be on PATH; the caller
#                    installs the pinned one with `unrepresentable-policy
#                    install-tool`, and puts it on PATH for that one command
#                    only). It uses the policy/zizmor.yml of the checkout this
#                    script comes from. An explicit --config makes zizmor load
#                    that file only and discover no other, so a zizmor.yml of
#                    the checked tree cannot switch audits off.
#   deny             cargo-deny licenses, bans and sources over the whole
#                    workspace, from the manifests and Cargo.lock only, so it
#                    runs offline (also in the sandboxed head-checks of pull
#                    requests). cargo-deny must be on PATH; the caller installs
#                    the pinned one with `install-tool`; in head-checks it is
#                    staged into the sandbox's `bin` directory, root-owned,
#                    after the step that still has the network. The rules are
#                    in deny.toml.
#   deny-advisories  cargo-deny advisories (vulnerabilities, yanked and
#                    unmaintained crates). It needs the RustSec advisory
#                    database, so it needs the network: it runs only where the
#                    network is allowed (the push jobs and the
#                    policy-sweep workflow, run on demand, because a new advisory can
#                    appear against a dependency that did not change), never in
#                    the offline head-checks, and it takes no --offline. A push
#                    skipped by `skip-check` also skips it; the sweep covers
#                    that case.
#
# The script runs from the repository root, or from a checkout of the code to
# check: the checks work on the current directory, and the script itself may
# come from another checkout (the head-checks job runs the script of the base
# checkout on the head checkout). EXTRA_CARGO_ARGS (for example --offline) go to
# every cargo command that takes them (owntree takes --range first); fmt,
# audit-workflows and deny-advisories ignore them (deny-advisories is never
# given --offline). Cargo runs only through ci/cargo.sh. This script sets
# neither CARGO_TARGET_DIR nor CARGO_HOME. An unknown or missing name exits 2.
#
# Tests in policy/src/workflow_checks.rs check that every check name used in a
# workflow exists here, that the push jobs run every check, that the head-checks
# job runs every check except deny-advisories, and that this script calls no
# bare `cargo`. EXPECTED_CHECKS in policy/src/headjobs/mod.rs lists the checks
# that head-checks finds in its status file; a test requires it to equal the
# checks the workflow runs.
set -eu

here=$(CDPATH='' cd "$(dirname "$0")" && pwd)

die() {
  printf 'ci/checks.sh: %s\n' "$1" >&2
  exit 2
}

cargo_sh() {
  sh "$here/cargo.sh" "$@"
}

usage() {
  printf '%s\n' \
    'usage: sh ci/checks.sh NAME [EXTRA_CARGO_ARGS...]' \
    'NAME is one of: fmt clippy clippy-main clippy-policy test policytest' \
    '  selftest canaries owntree package audit-workflows deny deny-advisories' \
    'owntree takes --range RANGE first. An unknown or missing name exits 2.' \
    'The header comment of this file describes each check.'
}

case "${1:-}" in
  -h | --help)
    usage
    exit 0
    ;;
esac

[ "$#" -ge 1 ] || die "usage: sh ci/checks.sh NAME [EXTRA_CARGO_ARGS...]"
name="$1"
shift

check_audit_workflows() {
  command -v zizmor >/dev/null 2>&1 || die "zizmor is not on PATH"
  # An explicit --config makes zizmor load that file only and discover none
  # (not even .github/zizmor.yml of the checked tree), so the checked tree
  # cannot switch audits off. The file is policy/zizmor.yml of the checkout
  # this script comes from (the base checkout in head-checks).
  zizmor --config "$here/../policy/zizmor.yml" --offline .github
}

check_canaries() {
  cargo_sh run "$@" --quiet -- canaries .
}

check_clippy_main() {
  cargo_sh clippy --all-targets "$@" -- -D warnings
}

check_clippy_policy() {
  cargo_sh clippy -p unrepresentable-policy --all-targets "$@" -- -D warnings
}

# licenses, bans and sources need no advisory database, so they work offline.
check_deny() {
  command -v cargo-deny >/dev/null 2>&1 || die "cargo-deny is not on PATH"
  cargo_sh deny --workspace "$@" check licenses bans sources
}

# Advisories need the advisory database: network, no --offline.
check_deny_advisories() {
  command -v cargo-deny >/dev/null 2>&1 || die "cargo-deny is not on PATH"
  cargo_sh deny --workspace check advisories
}

check_fmt() {
  cargo_sh fmt -p unrepresentable --check
  cargo_sh fmt -p unrepresentable-policy --check
}

# The default range: the last commit, or HEAD alone when there is none before.
default_range() {
  if git rev-parse --verify --quiet 'HEAD~1^{commit}' >/dev/null 2>&1; then
    printf '%s\n' 'HEAD~1..HEAD'
  else
    printf '%s\n' 'HEAD'
  fi
}

# Whether $1 is a range the owntree check accepts (see the header).
valid_range() {
  printf '%s\n' "$1" | grep -Eq '^(HEAD|HEAD~1\.\.HEAD|[0-9a-f]{40}\.\.\.?(HEAD|[0-9a-f]{40}))$'
}

check_owntree() {
  range=''
  if [ "${1:-}" = '--range' ]; then
    [ "$#" -ge 2 ] || die "--range needs a value"
    range="$2"
    shift 2
  fi
  [ -n "$range" ] || range="$(default_range)"
  valid_range "$range" || die "bad range '$range'"
  status=0
  cargo_sh run "$@" --quiet -- check --range "$range" . || status=$?
  # The tool exits 0 (clean), 1 (findings), 2 (could not decide) or 3 (a change
  # needs human approval). 1 and 2 fail this check, and so does any other
  # status (cargo itself failing). 3 passes: approval is the merge policy's
  # concern; for a push it was given by merging, for a pull request the
  # policy check asks for it. The approval lines are in the output above.
  case "$status" in
    0) ;;
    3) printf 'ci/checks.sh: owntree: the tool needs human approval for this range (exit 3); not a failure of this check\n' >&2 ;;
    *) exit "$status" ;;
  esac
}

# Refuses a packaged path that is a stray backup or editor file, or a file
# under src/ that is not Rust source. The list of paths (one per line, as
# `cargo package --list` prints it) is read from standard input. There is no
# allowed exception under src/: every file there is a .rs file. The one allowed
# exception elsewhere is Cargo.toml.orig, which `cargo package` writes itself.
refuse_junk_in_package() {
  junk=$(grep -E '(\.bak|\.orig|\.rej|\.swp|~)$|(^|/)\.DS_Store$|^src/.*[^/]$' | grep -Ev '^src/.*\.rs$|^Cargo\.toml\.orig$' || true)
  [ -z "$junk" ] || die "the package would contain a stray file: $(printf '%s' "$junk" | head -n 1)"
}

# `cargo publish --dry-run` verifies in the dev profile, which would create a
# `debug` directory. Package without verifying, then build the unpacked .crate
# in release mode with a target directory of its own in a scratch directory,
# run its `self-test` and check that what the tool needs is in it. `cargo
# package` refuses a tree with uncommitted changes, so on a dirty tree (a local
# run) `--allow-dirty` goes to `cargo package` alone, never to the other cargo
# commands; a clean checkout (CI) gets no such flag.
check_package() {
  version=$(sh "$here/version.sh" <Cargo.toml) || die "no [package] version in Cargo.toml"
  if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    cargo_sh package -p unrepresentable --list --allow-dirty "$@" | refuse_junk_in_package
    cargo_sh package -p unrepresentable --no-verify --allow-dirty "$@"
  else
    cargo_sh package -p unrepresentable --list "$@" | refuse_junk_in_package
    cargo_sh package -p unrepresentable --no-verify "$@"
  fi
  target=$(cargo_sh metadata --format-version 1 --no-deps "$@" |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
  [ -n "$target" ] || die "cannot find the target directory"
  crate="$target/package/unrepresentable-$version.crate"
  [ -f "$crate" ] || die "no package at $crate"
  scratch=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/unrepresentable-package.XXXXXX")
  trap 'rm -rf "$scratch"' EXIT
  tar -xzf "$crate" -C "$scratch"
  unpacked="$scratch/unrepresentable-$version"
  for needed in Cargo.toml Cargo.lock LICENSE-MIT LICENSE-APACHE README.md \
    clippy.toml rust-toolchain.toml deny.toml .unrepresentable/config \
    .unrepresentable/ledger.md .unrepresentable/canary-exemptions.md src/main.rs; do
    [ -f "$unpacked/$needed" ] || die "the package lacks $needed"
  done
  cargo_sh build "$@" \
    --manifest-path "$unpacked/Cargo.toml" \
    --target-dir "$scratch/target"
  "$scratch/target/release/unrepresentable" self-test
}

check_policytest() {
  cargo_sh test -p unrepresentable-policy "$@"
}

check_selftest() {
  cargo_sh run "$@" --quiet -- self-test .
}

check_test() {
  cargo_sh test "$@"
}

case "$name" in
  audit-workflows) check_audit_workflows ;;
  canaries) check_canaries "$@" ;;
  clippy)
    check_clippy_main "$@"
    check_clippy_policy "$@"
    ;;
  clippy-main) check_clippy_main "$@" ;;
  clippy-policy) check_clippy_policy "$@" ;;
  deny) check_deny "$@" ;;
  deny-advisories) check_deny_advisories ;;
  fmt) check_fmt ;;
  owntree) check_owntree "$@" ;;
  package) check_package "$@" ;;
  policytest) check_policytest "$@" ;;
  selftest) check_selftest "$@" ;;
  test) check_test "$@" ;;
  *) die "unknown check '$name'" ;;
esac
