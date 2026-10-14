#!/bin/sh
# stikk installer for Linux and macOS.
#
# It downloads the stikk release this script was published with, checks it against the checksum
# published beside it, and installs it into a directory you own. It installs nothing if that check
# fails. It can also install prikk -- the version control system stikk reads -- through prikk's own
# installer, if prikk is missing.
#
# Read it before you run it. That is why it is one file.
#
# Usage:
#   curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh
#   curl -fsSL https://github.com/prikk-vcs/stikk/releases/latest/download/install.sh | sh -s -- --no-prikk
#
# Options (environment variable in brackets):
#   --version X.Y.Z   the stikk version to install      [STIKK_INSTALL_VERSION]
#   --prefix DIR      where to put the binary           [STIKK_INSTALL_PREFIX], default ~/.local/bin
#   --with-prikk      install prikk if missing, without asking
#   --no-prikk        never install prikk
#   --base-url URL    download from a mirror instead    [STIKK_INSTALL_BASE_URL]
#
# What the checksum proves: the file you downloaded is the file the release published -- it was not
# altered on the way to you. What it does not prove: who built it. For that, see the note this
# script prints when it finishes.

set -eu

# Filled by release.yml at release time. @-delimited so a half-substituted script is obvious.
STIKK_VERSION="@STIKK_VERSION@"
PRIKK_PINNED_VERSION="@PRIKK_VERSION@"

REPO="prikk-vcs/stikk"
PRIKK_REPO="prikk-vcs/prikk"

version="${STIKK_INSTALL_VERSION:-$STIKK_VERSION}"
prefix="${STIKK_INSTALL_PREFIX:-$HOME/.local/bin}"
base_url="${STIKK_INSTALL_BASE_URL:-}"
want_prikk="ask"

say() { echo "stikk-install: $*"; }
die() { echo "stikk-install: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || die "--version needs a value"; version="$2"; shift 2 ;;
    --prefix)  [ $# -ge 2 ] || die "--prefix needs a value";  prefix="$2";  shift 2 ;;
    --base-url) [ $# -ge 2 ] || die "--base-url needs a value"; base_url="$2"; shift 2 ;;
    --with-prikk) want_prikk="yes"; shift ;;
    --no-prikk)   want_prikk="no";  shift ;;
    -h|--help) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

# The placeholders above are replaced at release time. Detect an unfilled one by its shape rather
# than by comparing against the literal -- the substitution would rewrite the comparison too, and
# the guard would then fire on every real install. (It did, the first time this was run.)
case "$version$PRIKK_PINNED_VERSION" in
  *@*)
    die "this is the template from the repository, not a published installer.
  Download it from a release:
    curl -fsSL https://github.com/$REPO/releases/latest/download/install.sh | sh" ;;
esac

# ---------------------------------------------------------------- which build do you need?

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64)          target="x86_64-unknown-linux-gnu" ;;
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

prikk_note() {
  say "stikk reads prikk repositories, so you need prikk too."
}

if command -v prikk >/dev/null 2>&1; then
  installed="$(prikk --version 2>/dev/null | tr -d '\r')"
  say "prikk is already installed: $installed"
  minor="$(echo "$installed" | sed -n 's/.*[^0-9]\([0-9][0-9]*\)\.\([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\2/p')"
  pinned_minor="$(echo "$PRIKK_PINNED_VERSION" | cut -d. -f2)"
  if [ -n "$minor" ] && [ "$minor" -ge 28 ] && [ "$minor" -le "$pinned_minor" ]; then
    say "that is inside the range this stikk was tested against (0.28 through $PRIKK_PINNED_VERSION)"
  else
    say "this stikk was tested against prikk 0.28 through $PRIKK_PINNED_VERSION. Yours still runs;"
    say "stikk will say its output has not been checked, rather than assume it has."
  fi
else
  prikk_note
  case "$os/$arch" in
    Linux/x86_64|Linux/aarch64|Linux/arm64|Darwin/arm64)
      if [ "$want_prikk" = "ask" ]; then
        if [ -t 0 ]; then
          printf 'stikk-install: install prikk %s now, with prikk own installer? [Y/n] ' "$PRIKK_PINNED_VERSION"
          read -r answer </dev/tty || answer=""
          case "$answer" in [Nn]*) want_prikk="no" ;; *) want_prikk="yes" ;; esac
        else
          # Piped from curl, so there is nobody to ask. Never block.
          want_prikk="no"
          say "no terminal to ask on, so prikk was not installed. Re-run with --with-prikk to install it."
        fi
      fi
      if [ "$want_prikk" = "yes" ]; then
        say "downloading prikk's own installer for $PRIKK_PINNED_VERSION"
        prikk_base="https://github.com/$PRIKK_REPO/releases/download/$PRIKK_PINNED_VERSION"
        curl -fsSL -o "$tmp/prikk-install.sh" "$prikk_base/install.sh" \
          || die "could not download prikk's installer. prikk was not installed; stikk was."
        curl -fsSL -o "$tmp/prikk-install.sh.sha256" "$prikk_base/install.sh.sha256" \
          || die "could not download the checksum for prikk's installer. prikk was not installed; stikk was."
        p_published="$(awk '{print $1}' < "$tmp/prikk-install.sh.sha256" | tr '[:upper:]' '[:lower:]')"
        p_computed="$(cd "$tmp" && $checksum_cmd prikk-install.sh | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
        if [ -z "$p_published" ] || [ "$p_published" != "$p_computed" ]; then
          die "prikk's installer does NOT match its published checksum. It was not run, and prikk
  was not installed. stikk is installed and will tell you prikk is missing."
        fi
        say "running prikk's installer (it checks prikk's own download the same way)"
        sh "$tmp/prikk-install.sh" --version "$PRIKK_PINNED_VERSION" --prefix "$prefix" \
          || say "prikk's installer did not finish. stikk is installed; install prikk yourself when you can."
      fi
      ;;
    *)
      say "prikk has no installer for $os $arch. Install it one of these ways:"
      if [ "$os" = "Darwin" ]; then
        say "  cargo install prikk --version $PRIKK_PINNED_VERSION"
      fi
      say "  download it from https://github.com/$PRIKK_REPO/releases/tag/$PRIKK_PINNED_VERSION"
      ;;
  esac
fi

# ---------------------------------------------------------------- what to do next

echo
say "done. Next:"
if [ "$on_path" -eq 0 ]; then
  say "  1. open a new terminal (PATH changed)"
  say "  2. make a repository:  prikk init"
  say "  3. open it:            stikk"
else
  say "  1. make a repository:  prikk init"
  say "  2. open it:            stikk"
fi
say "Press ? inside stikk for help."
echo
say "The checksum above proves this download is the file the release published. It does not prove"
say "who built it. For that, with GitHub CLI:"
say "  gh attestation verify <the archive> -R $REPO"
say "To remove stikk later:"
say "  curl -fsSL https://github.com/$REPO/releases/latest/download/uninstall.sh | sh"
