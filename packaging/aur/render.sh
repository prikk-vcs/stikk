#!/usr/bin/env bash
#
# Fill stikk's two AUR PKGBUILD templates for one published release.
#
#   render.sh <version> <outdir>
#
# Writes <outdir>/stikk/PKGBUILD and <outdir>/stikk-bin/PKGBUILD. **The one filler both workflows
# run** (RFC 051 decisions 3-5): `aur.yml` renders and lints on every change to `packaging/aur/**`
# and pushes nothing; `release.yml` calls the same workflow with `push: true` after the assets and
# the crates are published. A template is never first exercised by a release.
#
# **No number is written by hand**, the discipline `release.yml`'s installer job already holds:
#
# | value | where it comes from |
# |---|---|
# | the prikk floor | `stikk-prikk`'s own `print_version_matrix` example |
# | the crate's sha256 | the crates.io **index**, and the downloaded crate is checked against it |
# | each archive's sha256 | that release's own `.sha256` asset |
#
# **It refuses rather than guesses**: a version with no release, or not on crates.io, or a
# checksum that does not match what was downloaded, or a placeholder surviving substitution, each
# stop the run. A PKGBUILD is the thing a user's machine builds from; a wrong one is worse than
# none.
#
# It writes nothing outside <outdir> and installs nothing.
set -euo pipefail

die() { printf 'render.sh: %s\n' "$*" >&2; exit 1; }
note() { printf 'render.sh: %s\n' "$*"; }

[ "$#" -eq 2 ] || die "usage: render.sh <version> <outdir>"
version="$1"
outdir="$2"

case "$version" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) die "version '$version' is not a release version (expected <major>.<minor>.<patch>)" ;;
esac

here="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
root="$(CDPATH='' cd -- "$here/../.." && pwd)"
repo='prikk-vcs/stikk'

# ---------------------------------------------------------------- the prikk floor
#
# The same derivation the installers use (RFC 048): the number lives in `version.rs` and nowhere
# else, so raising the range there re-words the PKGBUILDs too. **It is not a pin on prikk** -- it
# is the lower bound of the range stikk has validated, and the AUR resolves prikk's own package.
read -r floor _ceiling < <(cargo run --quiet --manifest-path "$root/Cargo.toml" \
  -p stikk-prikk --example print_version_matrix)
[ -n "${floor:-}" ] || die "could not read the prikk floor from print_version_matrix"
note "prikk floor: 0.$floor"

# ---------------------------------------------------------------- the crate, and its checksum
#
# The index line for this exact version, then the crate itself checked against the `cksum` in it.
# Downloading without checking would make this script the weakest link in a chain whose whole
# point is that the AUR builds the bytes crates.io published.
index_url='https://index.crates.io/st/ik/stikk'
index_line="$(curl -fsSL "$index_url" | grep -F "\"vers\":\"$version\"" || true)"
[ -n "$index_line" ] || die "stikk $version is not in the crates.io index ($index_url)"
[ "$(printf '%s\n' "$index_line" | wc -l)" -eq 1 ] \
  || die "the crates.io index has more than one line for $version"
crate_sha="$(printf '%s\n' "$index_line" | sed -n 's/.*"cksum":"\([0-9a-f]\{64\}\)".*/\1/p')"
[ -n "$crate_sha" ] || die "no cksum in the crates.io index line for $version"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/stikk-$version.crate" \
  "https://static.crates.io/crates/stikk/stikk-$version.crate" \
  || die "could not download stikk-$version.crate from crates.io"
downloaded="$(sha256sum "$tmp/stikk-$version.crate" | cut -d' ' -f1)"
[ "$downloaded" = "$crate_sha" ] \
  || die "stikk-$version.crate does not match the index: index $crate_sha, downloaded $downloaded"
note "crate sha256: $crate_sha (index and download agree)"

# ---------------------------------------------------------------- the archives' checksums
#
# From the release's own `.sha256` assets, which `release.yml` writes beside each archive, rather
# than by hashing a download here: the published checksum is what every other consumer checks
# against, and reproducing it locally would hide a mismatch instead of showing one.
archive_sha() {
  local arch="$1" url line sha
  url="https://github.com/$repo/releases/download/$version/stikk-$version-$arch-unknown-linux-musl.tar.gz.sha256"
  line="$(curl -fsSL "$url" || true)"
  [ -n "$line" ] || die "no $arch checksum asset for $version ($url)"
  sha="$(printf '%s\n' "$line" | cut -d' ' -f1)"
  printf '%s' "$sha" | grep -Eq '^[0-9a-f]{64}$' \
    || die "the $arch checksum asset is not a sha256: '$line'"
  printf '%s' "$sha"
}
sha_x86_64="$(archive_sha x86_64)"
sha_aarch64="$(archive_sha aarch64)"
note "x86_64 sha256:  $sha_x86_64"
note "aarch64 sha256: $sha_aarch64"

# ---------------------------------------------------------------- fill
for pkg in stikk stikk-bin; do
  template="$here/$pkg/PKGBUILD.in"
  [ -f "$template" ] || die "missing template $template"
  mkdir -p "$outdir/$pkg"
  sed -e "s/@VERSION@/$version/g" \
      -e "s/@PRIKK_FLOOR@/$floor/g" \
      -e "s/@CRATE_SHA256@/$crate_sha/g" \
      -e "s/@SHA256_X86_64@/$sha_x86_64/g" \
      -e "s/@SHA256_AARCH64@/$sha_aarch64/g" \
    "$template" > "$outdir/$pkg/PKGBUILD"
  # No placeholder may survive into something a user's machine builds from.
  if grep -q '@[A-Z0-9_]\+@' "$outdir/$pkg/PKGBUILD"; then
    grep -n '@[A-Z0-9_]\+@' "$outdir/$pkg/PKGBUILD" >&2
    die "$pkg still holds a placeholder after substitution"
  fi
  note "wrote $outdir/$pkg/PKGBUILD"
done
