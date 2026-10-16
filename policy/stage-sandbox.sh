#!/bin/sh
# Stage the directory that sandbox.sh and offline.sh run in.
# usage: stage-sandbox.sh DIR [OPTION...]
#
#   --bin SRC=NAME        install the file SRC as DIR/bin/NAME (root-owned,
#                         readable and executable by every sandbox user, and
#                         on the PATH of every sandboxed command)
#   --private-bin UID:SRC=NAME
#                         install the file SRC as DIR/private-UID/NAME, owned
#                         by root and the group UID, mode 0750, in a directory
#                         of the same owner and mode. Only the sandbox user UID
#                         (65534 or 65533) can search that directory, so it can
#                         read and run the file and no other sandbox user can.
#                         The directory is not on any PATH; call the file by its
#                         full path. The differential check keeps the base
#                         binary under 65534 and the head binary under 65533.
#   --data SRC=NAME       install the file SRC as DIR/data/NAME (root-owned)
#   --cargo-home SRC      copy the cargo home SRC (crates fetched earlier) to
#                         DIR/cargo, owned by the sandbox user
#   --head SRC            copy the checkout SRC to DIR/work/head, owned by the
#                         sandbox user (it builds and tests there)
#   --head-as-caller SRC  copy the checkout SRC to DIR/work/head, owned by the
#                         caller and readable by everyone (a tool that makes it
#                         read-only runs on it)
#
# DIR must not exist and is made with sudo. It holds a root-owned copy of the
# toolchain (the sysroot of `rustc` for the RUSTUP_TOOLCHAIN the caller set),
# a copy of ci/checks.sh, ci/cargo.sh and ci/version.sh from this checkout, `bin`, `data`, and
# directories the sandbox user 65534 may write: home, cargo, mut and (with --head)
# work/head. Nothing the sandboxed code can write is read back as code or as
# instructions by the job; results are read from it as data.
set -eu

die() {
  printf 'stage-sandbox.sh: %s\n' "$1" >&2
  exit 2
}

here=$(CDPATH='' cd "$(dirname "$0")" && pwd)
ci="$here/../ci"

[ "$#" -ge 1 ] || die "usage: stage-sandbox.sh DIR [OPTION...]"
dir="$1"
shift
case "$dir" in
  /*) ;;
  *) die "DIR must be an absolute path" ;;
esac
case "$dir" in
  *[!A-Za-z0-9/._-]*) die "DIR has a character outside A-Za-z0-9/._-" ;;
esac
[ ! -e "$dir" ] || die "$dir exists"
command -v rustc >/dev/null 2>&1 || die "rustc is not on PATH"
sysroot="$(rustc --print sysroot)"
[ -d "$sysroot/bin" ] || die "no toolchain at $sysroot"

sudo install -d -m 0755 "$dir"
sudo cp -a "$sysroot" "$dir/toolchain"
sudo chmod -R go+rX,go-w "$dir/toolchain"
sudo install -d -m 0755 "$dir/bin" "$dir/ci" "$dir/data" "$dir/work"
sudo install -m 0755 "$ci/checks.sh" "$ci/cargo.sh" "$ci/version.sh" "$dir/ci/"
sudo install -d -m 0755 -o 65534 -g 65534 "$dir/home" "$dir/cargo" "$dir/mut"

# Splits SRC=NAME into $src and $name; NAME is a plain file name.
split_pair() {
  src="${1%%=*}"
  name="${1#*=}"
  [ "$src" != "$1" ] || die "expected SRC=NAME, got $1"
  case "$name" in
    '' | . | .. | */* | *[!A-Za-z0-9._-]*) die "bad name '$name'" ;;
  esac
  [ -f "$src" ] || die "$src is not a file"
}

headed=no
while [ "$#" -gt 0 ]; do
  option="$1"
  shift
  [ "$#" -ge 1 ] || die "$option needs a value"
  value="$1"
  shift
  case "$option" in
    --bin)
      split_pair "$value"
      sudo install -m 0755 "$src" "$dir/bin/$name"
      ;;
    --data)
      split_pair "$value"
      sudo install -m 0644 "$src" "$dir/data/$name"
      ;;
    --private-bin)
      uid="${value%%:*}"
      [ "$uid" != "$value" ] || die "expected UID:SRC=NAME, got $value"
      case "$uid" in
        65534 | 65533) ;;
        *) die "the user of --private-bin must be 65534 or 65533, not '$uid'" ;;
      esac
      split_pair "${value#*:}"
      sudo install -d -m 0750 -o root -g "$uid" "$dir/private-$uid"
      sudo install -m 0750 -o root -g "$uid" "$src" "$dir/private-$uid/$name"
      ;;
    --cargo-home)
      [ -d "$value" ] || die "$value is not a directory"
      sudo cp -a "$value/." "$dir/cargo/"
      sudo chown -R 65534:65534 "$dir/cargo"
      ;;
    --head)
      [ "$headed" = no ] || die "one of --head and --head-as-caller"
      headed=yes
      [ -d "$value" ] || die "$value is not a directory"
      sudo cp -a "$value" "$dir/work/head"
      sudo chown -R 65534:65534 "$dir/work/head"
      ;;
    --head-as-caller)
      [ "$headed" = no ] || die "one of --head and --head-as-caller"
      headed=yes
      [ -d "$value" ] || die "$value is not a directory"
      sudo cp -a "$value" "$dir/work/head"
      sudo chown -R "$(id -u):$(id -g)" "$dir/work/head"
      sudo chmod -R go+rX "$dir/work/head"
      ;;
    *) die "unknown option $option" ;;
  esac
done
