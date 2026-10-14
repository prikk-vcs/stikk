#!/bin/sh
# stikk uninstaller for Linux and macOS.
#
# It removes the stikk binary the installer put in place, and the PATH block the installer added to
# your shell's startup file. It removes nothing else.
#
# It never removes prikk. prikk has its own uninstaller, and this script prints how to run it.
#
# Usage:
#   curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/uninstall.sh | sh
#
# Options (environment variable in brackets):
#   --prefix DIR   where stikk was installed   [STIKK_INSTALL_PREFIX], default ~/.local/bin

set -eu

REPO="prikk-vcs/stikk"
PRIKK_REPO="prikk-vcs/prikk"

prefix="${STIKK_INSTALL_PREFIX:-$HOME/.local/bin}"

say() { echo "stikk-uninstall: $*"; }
die() { echo "stikk-uninstall: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) [ $# -ge 2 ] || die "--prefix needs a value"; prefix="$2"; shift 2 ;;
    -h|--help) sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

removed_anything=0

if [ -f "$prefix/stikk" ]; then
  rm -f "$prefix/stikk" || die "could not remove $prefix/stikk"
  say "removed $prefix/stikk"
  removed_anything=1
else
  say "no stikk binary at $prefix/stikk (nothing to remove there)"
fi

path_begin="# >>> stikk installer >>>"
path_end="# <<< stikk installer <<<"

for rc_file in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
  [ -f "$rc_file" ] || continue
  grep -qF "$path_begin" "$rc_file" || continue
  tmp="$(mktemp)"
  # Drop the marked block, inclusive, **and the single blank line the installer wrote before it**,
  # so the file goes back to exactly what it was. Blank lines that were already there are kept:
  # they are held until we know what follows, and only one is dropped, immediately before the
  # block. (Without this the file gained a blank line on every install/uninstall round trip.)
  awk -v b="$path_begin" -v e="$path_end" '
    $0 == b {
      skip = 1
      if (held > 0) held--
      while (held > 0) { print ""; held-- }
      next
    }
    skip { if ($0 == e) skip = 0; next }
    $0 == "" { held++; next }
    { while (held > 0) { print ""; held-- } print }
    END { while (held > 0) { print ""; held-- } }
  ' "$rc_file" > "$tmp" && mv "$tmp" "$rc_file"
  say "removed the PATH block from $rc_file"
  removed_anything=1
done

if [ "$removed_anything" -eq 0 ]; then
  say "nothing of stikk's was found. If you installed it somewhere else, pass --prefix DIR."
fi

echo
say "done. stikk's config and state were not touched; 'stikk config path' showed where they live."
say "Your repositories are untouched: stikk never wrote to them."
echo
say "prikk was NOT removed -- it is a separate program with its own uninstaller:"
say "  curl -fsSL https://github.com/$PRIKK_REPO/releases/latest/download/uninstall.sh | sh"
say "To install stikk again:"
say "  curl -fsSL https://github.com/$REPO/releases/latest/download/install.sh | sh"
