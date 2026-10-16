#!/bin/sh
# The cargo entry point for this repository's workflows and local use.
#
# usage: sh ci/cargo.sh SUBCOMMAND [ARGS...]
#        sh ci/cargo.sh -h | --help
#
# Purpose: every cargo call of CI goes through this script, so that the flags
# below are applied in one place. Cargo runs with the release profile and with
# an unchanged lock file, so no `debug` profile directory is created and
# Cargo.lock is not rewritten:
#   build check clippy doc run test  ->  cargo SUBCOMMAND --release --locked ARGS
#   fetch metadata package publish   ->  cargo SUBCOMMAND --locked ARGS
#                                        (no --release flag exists; the
#                                        verification build of package/publish
#                                        stays under target/package)
#   deny                             ->  cargo deny --locked ARGS (cargo-deny reads
#                                        the manifests and Cargo.lock; it builds
#                                        nothing, so no target directory is made)
#   fmt                              ->  cargo fmt ARGS, only with -p/--package;
#                                        --all is refused (it would reformat
#                                        canaries/, whose `//~` marks must not move)
# Any other subcommand exits 2. Extra cargo flags (--offline, --quiet, -p NAME,
# --target T) go in ARGS. The script sets neither CARGO_TARGET_DIR nor
# CARGO_HOME, and it is not part of the consumer installer.
#
# Related fact: the cargo calls that the tool itself makes go through
# `process::cargo` in src/process.rs, the only place that spawns cargo. It
# inserts `--release --locked` after the subcommand.
set -eu

die() {
  printf 'ci/cargo.sh: %s\n' "$1" >&2
  exit 2
}

usage() {
  printf '%s\n' \
    'usage: sh ci/cargo.sh SUBCOMMAND [ARGS...]' \
    '  build check clippy doc run test  add --release --locked' \
    '  fetch metadata package publish   add --locked' \
    '  deny                             add --locked' \
    '  fmt                              requires -p NAME; --all is refused' \
    'Other subcommands exit 2. The header comment of this file has the details.'
}

case "${1:-}" in
  -h | --help)
    usage
    exit 0
    ;;
esac

[ "$#" -ge 1 ] || die "usage: sh ci/cargo.sh SUBCOMMAND [ARGS...]"
subcommand="$1"
shift

case "$subcommand" in
  build | check | clippy | doc | run | test)
    exec cargo "$subcommand" --release --locked "$@"
    ;;
  fetch | metadata | package | publish)
    exec cargo "$subcommand" --locked "$@"
    ;;
  deny)
    exec cargo deny --locked "$@"
    ;;
  fmt)
    scoped=no
    for argument in "$@"; do
      case "$argument" in
        --)
          break
          ;;
        --all | --workspace)
          die "cargo fmt must name a package with -p; --all would reformat canaries/"
          ;;
        -p | --package | -p* | --package=*)
          scoped=yes
          ;;
      esac
    done
    [ "$scoped" = yes ] || die "cargo fmt must name a package with -p NAME"
    exec cargo fmt "$@"
    ;;
  *)
    die "subcommand '$subcommand' is not allowed"
    ;;
esac
