#!/bin/sh
# stikk installer for Linux and macOS.
#
# It downloads the stikk release this script was published with, checks it against the checksum
# published beside it, and installs it into a directory you own. It installs nothing if that check
# fails.
#
# It installs stikk and nothing else. stikk reads prikk repositories, so it also *checks* whether
# prikk is there and new enough -- and when it is not, prints prikk's own install command. prikk
# installs itself: where it lives, how it is verified and which version you get are prikk's to
# decide, in one place, for every tool built on it.
#
# Read it before you run it. That is why it is one file.
#
# Usage:
#   curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh
#
# Options (environment variable in brackets):
#   --version X.Y.Z   the stikk version to install      [STIKK_INSTALL_VERSION]
#   --prefix DIR      where to put the binary           [STIKK_INSTALL_PREFIX], default ~/.local/bin
#   --base-url URL    download from a mirror instead    [STIKK_INSTALL_BASE_URL]
#
# What the checksum proves: the file you downloaded is the file the release published -- it was not
# altered on the way to you. What it does not prove: who built it. For that, see the note this
# script prints when it finishes.

set -eu

# Filled by release.yml at release time. @-delimited so a half-substituted script is obvious.
#
# **Neither of these is a pin.** stikk installs no prikk, so no version here is ever passed to
# anything; they word the version report below and nothing else. Both are derived from
# `stikk_prikk::version`'s own bounds at release time, so raising the range in one Rust file
# re-words these scripts automatically and no number is written twice.
STIKK_VERSION="@STIKK_VERSION@"
PRIKK_FLOOR_MINOR="@PRIKK_FLOOR@"
PRIKK_TESTED_CEILING="@PRIKK_VERSION@"

REPO="prikk-vcs/stikk"

# prikk's own install command and guide, printed when prikk is missing or too old. **Not
# downloaded, not run** -- printed, for the reader to run themselves.
#
# **prikk.org, not the GitHub Pages address** (review v1 C1): measured, the Pages URL answers 301
# to *plain http* at prikk.org, and sending a newcomer from https to http is not something this
# script should do on prikk's behalf. prikk.org is prikk's canonical site and answers over https
# directly.
PRIKK_INSTALL_CMD="curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh"
PRIKK_INSTALL_GUIDE="https://prikk.org/docs/guide/install.html"
STIKK_RELEASES="https://github.com/$REPO/releases/latest"

version="${STIKK_INSTALL_VERSION:-$STIKK_VERSION}"
prefix="${STIKK_INSTALL_PREFIX:-$HOME/.local/bin}"
base_url="${STIKK_INSTALL_BASE_URL:-}"

say() { echo "stikk-install: $*"; }
die() { echo "stikk-install: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || die "--version needs a value"; version="$2"; shift 2 ;;
    --prefix)  [ $# -ge 2 ] || die "--prefix needs a value";  prefix="$2";  shift 2 ;;
    --base-url) [ $# -ge 2 ] || die "--base-url needs a value"; base_url="$2"; shift 2 ;;
    -h|--help) sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

# The placeholders above are replaced at release time. Detect an unfilled one by its shape rather
# than by comparing against the literal -- the substitution would rewrite the comparison too, and
# the guard would then fire on every real install. (It did, the first time this was run.)
case "$version$PRIKK_FLOOR_MINOR$PRIKK_TESTED_CEILING" in
  *@*)
    die "this is the template from the repository, not a published installer.
  Download it from a release:
    curl -fsSL https://github.com/$REPO/releases/latest/download/install.sh | sh" ;;
esac

# ---------------------------------------------------------------- which build do you need?

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  # **The static musl build on x86_64 Linux, not the glibc one.** Measured: the musl archive is
  # static-pie with no interpreter and runs in an empty chroot with no libc at all, while the glibc
  # build there fails with "No such file or directory" -- which names the missing *loader*, not the
  # file, and is the one error the person this installer is for cannot diagnose. The glibc build
  # also refuses to start on Alpine, or against a glibc older than the one it was built on. Both
  # builds produce identical output on an ordinary glibc system (checked, including a real
  # orientation). The manual table in the README still offers both.
  Linux/x86_64)          target="x86_64-unknown-linux-musl" ;;
  # aarch64 has no musl build published, so it stays glibc.
  Linux/aarch64|Linux/arm64) target="aarch64-unknown-linux-gnu" ;;
  Darwin/arm64)          target="aarch64-apple-darwin" ;;
  Darwin/x86_64)         target="x86_64-apple-darwin" ;;
  *)
    die "no prebuilt stikk for $os $arch.
  Download one by hand from https://github.com/$REPO/releases/latest
  or build it with:  cargo install stikk" ;;
esac

archive="stikk-$version-$target.tar.gz"
[ -n "$base_url" ] || base_url="https://github.com/$REPO/releases/download/$version"

for tool in curl tar; do
  command -v "$tool" >/dev/null || die "$tool is needed and was not found"
done
if command -v sha256sum >/dev/null; then
  checksum_cmd="sha256sum"
elif command -v shasum >/dev/null; then
  checksum_cmd="shasum -a 256"
else
  die "no sha256sum or shasum found, so the download cannot be checked. Nothing was installed."
fi

# ---------------------------------------------------------------- download, check, install

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "downloading $archive ($version, $target)"
curl -fsSL -o "$tmp/$archive" "$base_url/$archive" \
  || die "could not download $base_url/$archive"
curl -fsSL -o "$tmp/$archive.sha256" "$base_url/$archive.sha256" \
  || die "could not download the checksum for $archive"

say "checking the download"
# The checksum file holds "<hash>  <file name>"; the name in it may not match where we saved it,
# so compare the hash itself rather than trusting the name.
published="$(awk '{print $1}' < "$tmp/$archive.sha256" | tr '[:upper:]' '[:lower:]')"
computed="$(cd "$tmp" && $checksum_cmd "$archive" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
if [ -z "$published" ] || [ "$published" != "$computed" ]; then
  die "the download does NOT match the published checksum.
  expected: $published
  got:      $computed
  Nothing was installed. Try again; if it keeps failing, do not use the file."
fi

say "extracting"
tar -xzf "$tmp/$archive" -C "$tmp" || die "could not extract $archive"
[ -f "$tmp/stikk-$version-$target/stikk" ] || die "the archive did not contain stikk where expected"

mkdir -p "$prefix" || die "could not create $prefix"
# Overwriting is what makes re-running this safe.
cp "$tmp/stikk-$version-$target/stikk" "$prefix/stikk" || die "could not write to $prefix"
chmod 755 "$prefix/stikk"
say "installed to $prefix/stikk"

# ---------------------------------------------------------------- PATH

path_begin="# >>> stikk installer >>>"
path_end="# <<< stikk installer <<<"
on_path=0
case ":$PATH:" in *":$prefix:"*) on_path=1 ;; esac

if [ "$on_path" -eq 0 ]; then
  shell_name="$(basename "${SHELL:-sh}")"
  case "$shell_name" in
    zsh)  rc_file="$HOME/.zshrc" ;;
    bash) rc_file="$HOME/.bashrc" ;;
    *)    rc_file="" ;;
  esac
  if [ -n "$rc_file" ]; then
    if [ -f "$rc_file" ] && grep -qF "$path_begin" "$rc_file"; then
      say "$rc_file already has a PATH entry from a previous install"
    else
      # The literal $PATH below lands in the file unexpanded on purpose, so it re-evaluates at every
      # future shell start rather than freezing today's value.
      # shellcheck disable=SC2016  # that is exactly why it is in single quotes
      printf '\n%s\nexport PATH="%s:$PATH"\n%s\n' "$path_begin" "$prefix" "$path_end" >> "$rc_file"
      say "added $prefix to PATH in $rc_file"
      say "open a new terminal, or run: . $rc_file"
    fi
    say "note for macOS Terminal: it starts a login shell, so it reads the .bash_profile in your"
    say "home directory rather than .bashrc, unless .bash_profile already sources it"
  else
    say "$prefix is not on your PATH. Add this to your shell's startup file:"
    say "  export PATH=\"$prefix:\$PATH\""
  fi
fi

# ---------------------------------------------------------------- prikk

# **stikk installs no prikk.** It checks, and when prikk is missing or too old it prints prikk's
# own command for the reader to run. Nothing below downloads, verifies, pins or runs anything of
# prikk's: each program installs itself (RFC 048 decision 2), so prikk's version, its location
# and what its checksum claims stay prikk's decisions, made once, for every tool built on it.
#
# **The platform is not checked here either.** prikk's own installer decides what prikk supports
# and says so itself -- on an Intel Mac it refuses with prikk's own advice. A list of prikk's
# platforms kept in this file would be a second copy to go stale.
#
# Whatever this reports, stikk is installed and this script exits 0: a missing prikk is a thing
# to tell you about, not a failure of the install you asked for.
# **What the reader has to do about prikk, if anything** -- empty when nothing. Held as the
# sentence itself rather than a flag, because the five rows do not all ask for the same thing:
# a missing prikk is installed, an old one is updated, and a prikk from another major is answered
# by a newer *stikk*.
prikk_next_step=""

prikk_how_to_get_it() {
  say "prikk installs itself. Run this:"
  say "  $PRIKK_INSTALL_CMD"
  say "prikk's install guide: $PRIKK_INSTALL_GUIDE"
}

if command -v prikk >/dev/null 2>&1; then
  installed="$(prikk --version 2>/dev/null | tr -d '\r')"
  say "prikk is already installed: $installed"
  major="$(echo "$installed" | sed -n 's/.*[^0-9]\([0-9][0-9]*\)\.\([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1/p')"
  minor="$(echo "$installed" | sed -n 's/.*[^0-9]\([0-9][0-9]*\)\.\([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\2/p')"
  ceiling_minor="$(echo "$PRIKK_TESTED_CEILING" | cut -d. -f2)"
  if [ -z "$minor" ]; then
    # **Measured** (review v1 C2): stikk's handshake parses this line, and a version it cannot
    # read makes stikk exit with *could not find a version in prikk output*. So this row says
    # what happens, and must not borrow the reassuring sentence the two in-range rows use.
    prikk_next_step="install or update prikk (the command is above)"
    say "stikk could not read a version number from that, and will refuse to start with this prikk."
    prikk_how_to_get_it
  elif [ "$major" != "0" ]; then
    # **A prikk from another major is not "too old"** (review v1 C3). stikk supports major 0 only
    # (`Version::is_supported`), so the fix is a newer *stikk*: pointing at prikk's installer
    # would leave the reader on the prikk they already have. Measured: stikk opens such a
    # repository **read-only** rather than refusing.
    prikk_next_step="update stikk (see above)"
    say "this stikk was built for prikk 0.x and does not know prikk $major.x. It will open your"
    say "repository read-only and change nothing. A newer stikk will know it:"
    say "  $STIKK_RELEASES"
  elif [ "$minor" -lt "$PRIKK_FLOOR_MINOR" ]; then
    # **Below the floor is the one 0.x row that is not "yours still runs".** Measured: stikk
    # opens read-only here too, so that is what it says rather than leaving it to be discovered.
    prikk_next_step="install or update prikk (the command is above)"
    say "that is too old for this stikk, which needs prikk 0.$PRIKK_FLOOR_MINOR or newer. stikk will open your"
    say "repository read-only until you update prikk."
    prikk_how_to_get_it
  elif [ "$minor" -le "$ceiling_minor" ]; then
    say "that is inside the range this stikk was tested against (0.$PRIKK_FLOOR_MINOR through $PRIKK_TESTED_CEILING)"
  else
    # **Calm on purpose.** This is the normal state for a few days after every prikk release, and
    # a newcomer must not read it as a fault.
    say "that is newer than this stikk was tested against (0.$PRIKK_FLOOR_MINOR through $PRIKK_TESTED_CEILING)."
    say "It still runs; stikk will say its output has not been checked, rather than assume it has."
  fi
else
  prikk_next_step="install prikk (the command is above)"
  say "stikk reads prikk repositories, so you need prikk too."
  prikk_how_to_get_it
fi

# ---------------------------------------------------------------- what to do next

echo
say "done. Next:"
# **The steps are numbered as they are actually needed**, so a reader never has to work out which
# of them apply to them. A missing prikk is step 1 because `prikk init` cannot run without it.
step=1
if [ -n "$prikk_next_step" ]; then
  say "  $step. $prikk_next_step"
  step=$((step + 1))
fi
if [ "$on_path" -eq 0 ]; then
  say "  $step. open a new terminal (PATH changed)"
  step=$((step + 1))
fi
say "  $step. make a repository:  prikk init"
step=$((step + 1))
say "  $step. open it:            stikk"
say "Press ? inside stikk for help."
echo
say "The checksum above proves this download is the file the release published. It does not prove"
say "who built it. For that, with GitHub CLI:"
say "  gh attestation verify <the archive> -R $REPO"
say "To remove stikk later:"
say "  curl -fsSL https://github.com/$REPO/releases/latest/download/uninstall.sh | sh"
