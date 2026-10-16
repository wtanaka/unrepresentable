#!/bin/sh
# Copy one regular file out of the part of the sandbox directory that the
# sandboxed code could write, refusing anything that is not that file.
# usage: take-file.sh ROOT SRC DEST
#
# ROOT is the directory the file must be in once every link in its path is
# resolved (for example /opt/sandbox/work). Head code can leave a link where
# the file should be, or swap a directory for a link while it is being read;
# copying what the link leads to would put a file of the runner into an
# artifact.
#
# The file is opened once, and every check is made on that open file, not on
# its path: its type (`test -f` on /proc/PID/fd/3 asks about the inode that was
# opened), where it really is (the link /proc/PID/fd/3 points at it) and its
# size (at most 256 MiB, counted while reading). There is no check of a path
# followed by a separate copy of the path, so nothing can be swapped between
# the two. Linux only (/proc). The caller must also have ended every process of
# the sandbox user first (policy/sandbox.sh does that before it returns); this
# script does not rely on it for the checks above.
set -eu

# Everything below is bounded in time: opening a FIFO that head code left at
# the path would otherwise block until the job times out. The type check
# further down refuses that before the open; this limit covers a path that
# changes type between the check and the open.
if [ "${TAKE_FILE_BOUNDED:-}" != 1 ]; then
  exec timeout 120 env TAKE_FILE_BOUNDED=1 sh "$0" "$@"
fi

die() {
  printf 'take-file.sh: %s\n' "$1" >&2
  exit 1
}

limit=268435456

[ "$#" -eq 3 ] || die "usage: take-file.sh ROOT SRC DEST"
root="$(realpath -e -- "$1")" || die "$1 does not exist"
src="$2"
dest="$3"
[ ! -e "$dest" ] && [ ! -L "$dest" ] || die "$dest exists"
[ -e "$src" ] || die "$src does not exist"
# Before opening: opening a FIFO for reading blocks until something writes to
# it, and a link is not wanted here at all. The checks on the open file below
# stay: these only keep the open from blocking.
{ [ -f "$src" ] && [ ! -L "$src" ] && [ ! -p "$src" ]; } || die "$src is not a plain regular file"
exec 3<"$src"
open="/proc/$$/fd/3"
real="$(readlink -- "$open")" || die "cannot tell what $src opened"
case "$real" in
  "$root"/*) ;;
  *) die "$src leads to $real, outside $root" ;;
esac
[ -f "$open" ] || die "$real is not a regular file"
head -c "$((limit + 1))" <&3 >"$dest" || {
  rm -f -- "$dest"
  die "cannot read $real"
}
exec 3<&-
size="$(wc -c <"$dest")"
if [ "$size" -gt "$limit" ]; then
  rm -f -- "$dest"
  die "$real is larger than $limit bytes"
fi
