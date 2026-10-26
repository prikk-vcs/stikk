<#
.SYNOPSIS
  stikk installer for Windows.

.DESCRIPTION
  Downloads the stikk release this script was published with, checks it against the checksum
  published beside it, and installs it into a directory you own. It installs nothing if that check
  fails.

  It installs stikk and nothing else. stikk reads prikk repositories, so it also *checks* whether
  prikk is there and new enough -- and when it is not, says where to get it. prikk installs
  itself: where it lives, how it is verified and which version you get are prikk's to decide, in
  one place, for every tool built on it.

  Read it before you run it. That is why it is one file.

.EXAMPLE
  irm https://github.com/prikk-vcs/stikk/releases/latest/download/install.ps1 | iex

.EXAMPLE
  & ([scriptblock]::Create((irm https://github.com/prikk-vcs/stikk/releases/latest/download/install.ps1))) -Prefix D:\tools
#>
# Write-Host is deliberate: this script talks to a person at a console, and `irm | iex` leaves
# no pipeline for Write-Output to go to. Suppressed here rather than per line.
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '',
  Justification = 'This is an interactive installer; its output is for a human, not a pipeline.')]
[CmdletBinding()]
param(
  # The stikk version to install.
  [string] $Version   = $env:STIKK_INSTALL_VERSION,
  # Where to put stikk.exe.
  [string] $Prefix    = $env:STIKK_INSTALL_PREFIX,
  # Download from a mirror instead.
  [string] $BaseUrl   = $env:STIKK_INSTALL_BASE_URL
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Filled by release.yml at release time. @-delimited so a half-substituted script is obvious.
#
# **Neither of these is a pin.** stikk installs no prikk, so no version here is ever passed to
# anything; they word the version report below and nothing else. Both are derived from
# `stikk_prikk::version`'s own bounds at release time, so raising the range in one Rust file
# re-words these scripts automatically and no number is written twice.
$StikkVersion        = '@STIKK_VERSION@'
$PrikkFloorMinor     = '@PRIKK_FLOOR@'
$PrikkTestedCeiling  = '@PRIKK_VERSION@'

$Repo      = 'prikk-vcs/stikk'
$PrikkRepo = 'prikk-vcs/prikk'

# Where to send someone who needs prikk. **prikk's own command, printed and never run.** prikk
# publishes a Windows installer from 0.54.1 (RFC 052 F3), so this says the one line prikk
# publishes and links prikk's guide beside it -- the two lines `install.sh` already prints on
# Linux and macOS. This script fetches nothing of prikk's and runs nothing of prikk's (RFC 048
# decision 2): the reader runs that line themselves, in their own session.
#
# **prikk.org, not the GitHub Pages address** (review v1 C1): measured, the Pages URL answers 301
# to *plain http* at prikk.org, and sending a newcomer from https to http is not something this
# script should do on prikk's behalf.
$PrikkInstallCmd   = "irm https://github.com/$PrikkRepo/releases/latest/download/install.ps1 | iex"
$PrikkInstallGuide = 'https://prikk.org/docs/guide/install.html'
$StikkReleases     = "https://github.com/$Repo/releases/latest"

function Say  { param([string] $Message) Write-Host "stikk-install: $Message" }
function Die  { param([string] $Message) Write-Host "stikk-install: $Message" -ForegroundColor Red; exit 1 }

if (-not $Version) { $Version = $StikkVersion }
if (-not $Prefix)  { $Prefix  = Join-Path $env:LOCALAPPDATA 'Programs\stikk' }

# The placeholders above are replaced at release time. Detect an unfilled one by its shape rather
# than by comparing against the literal -- the substitution would rewrite the comparison too, and
# the guard would then fire on every real install.
if (($Version + $PrikkFloorMinor + $PrikkTestedCeiling) -like '*@*') {
  Die @"
this is the template from the repository, not a published installer.
  Download it from a release:
    irm https://github.com/$Repo/releases/latest/download/install.ps1 | iex
"@
}

# ---------------------------------------------------------------- which build do you need?

# stikk publishes one Windows build, for 64-bit Intel/AMD. It runs on ARM Windows through emulation.
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -notin @('AMD64', 'ARM64', 'x86')) {
  Die "unexpected processor architecture '$arch'. Download by hand from https://github.com/$Repo/releases/latest"
}
$target  = 'x86_64-pc-windows-msvc'
$archive = "stikk-$Version-$target.zip"
if (-not $BaseUrl) { $BaseUrl = "https://github.com/$Repo/releases/download/$Version" }

# ---------------------------------------------------------------- download, check, install

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("stikk-install-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  $archivePath  = Join-Path $tmp $archive
  $checksumPath = "$archivePath.sha256"

  Say "downloading $archive ($Version, $target)"
  # A mirror may be a local directory. `Invoke-WebRequest` refuses the `file` scheme outright
  # (measured: "The 'file' scheme is not supported"), so a local source is copied instead. This is
  # also how the installer's own tests serve a deliberately tampered archive.
  $localBase = $null
  if ($BaseUrl -match '^file://') {
    $localBase = ([System.Uri] $BaseUrl).LocalPath
  } elseif (Test-Path -LiteralPath $BaseUrl -PathType Container) {
    $localBase = (Resolve-Path -LiteralPath $BaseUrl).Path
  }
  try {
    if ($localBase) {
      Copy-Item -LiteralPath (Join-Path $localBase $archive)          -Destination $archivePath  -Force
      Copy-Item -LiteralPath (Join-Path $localBase "$archive.sha256") -Destination $checksumPath -Force
    } else {
      Invoke-WebRequest -Uri "$BaseUrl/$archive" -OutFile $archivePath -UseBasicParsing
      Invoke-WebRequest -Uri "$BaseUrl/$archive.sha256" -OutFile $checksumPath -UseBasicParsing
    }
  } catch {
    Die "could not fetch from $BaseUrl -- $($_.Exception.Message)"
  }

  Say 'checking the download'
  # The checksum file holds "<hash>  <file name>". Take the hash; the name in it may differ from
  # where we saved the file.
  $published = ((Get-Content $checksumPath -Raw).Trim() -split '\s+')[0]
  $computed  = (Get-FileHash $archivePath -Algorithm SHA256).Hash
  if ([string]::IsNullOrWhiteSpace($published) -or ($published -ne $computed)) {
    Die @"
the download does NOT match the published checksum.
  expected: $published
  got:      $computed
  Nothing was installed. Try again; if it keeps failing, do not use the file.
"@
  }

  Say 'extracting'
  Expand-Archive -Path $archivePath -DestinationPath $tmp -Force
  $exe = Join-Path $tmp "stikk-$Version-$target\stikk.exe"
  if (-not (Test-Path $exe)) { Die 'the archive did not contain stikk.exe where expected' }

  if (-not (Test-Path $Prefix)) { New-Item -ItemType Directory -Path $Prefix -Force | Out-Null }
  # Overwriting is what makes re-running this safe.
  Copy-Item $exe (Join-Path $Prefix 'stikk.exe') -Force
  Say "installed to $(Join-Path $Prefix 'stikk.exe')"
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

# ---------------------------------------------------------------- PATH

# The USER Path only. Never the machine-wide one: this script installs for you, not for everyone.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not $userPath) { $userPath = '' }
$already = ($userPath -split ';' | Where-Object { $_ -and ($_.TrimEnd('\') -ieq $Prefix.TrimEnd('\')) })
if (-not $already) {
  $newPath = if ($userPath.TrimEnd(';')) { $userPath.TrimEnd(';') + ';' + $Prefix } else { $Prefix }
  [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
  Say "added $Prefix to your user PATH"
} else {
  Say "$Prefix is already on your user PATH"
}

# **And this session's own PATH, so `stikk` works in the window you are standing in**
# (RFC 048 decision 3). `irm ... | iex` runs inside the user's session, so the installer can do
# this and the "open a new PowerShell window" step disappears -- it was the last thing between a
# newcomer and a working command. (On Linux and macOS the installer runs in a child shell and
# cannot reach the parent's environment, so that step stays there.)
#
# **Decided on this session, not on the user PATH.** A window opened before an earlier install
# has the folder in the user PATH and not in `$env:Path`, and keying this off the user PATH
# would leave exactly that window unable to find stikk.
$inSession = ($env:Path -split ';' | Where-Object { $_ -and ($_.TrimEnd('\') -ieq $Prefix.TrimEnd('\')) })
if (-not $inSession) {
  # Prepended: this stikk is the one just installed, and it should win over any older copy
  # further along the path.
  $env:Path = "$Prefix;$env:Path"
}

# ---------------------------------------------------------------- prikk

# **stikk installs no prikk.** It checks, and when prikk is missing or too old it says where to
# get it. Nothing below downloads, verifies, pins or runs anything of prikk's: each program
# installs itself (RFC 048 decision 2), so prikk's version, its location and what its checksum
# claims stay prikk's decisions, made once, for every tool built on it.
#
# Whatever this reports, stikk is installed and this script exits 0.
# **What the reader has to do about prikk, if anything** -- empty when nothing. Held as the
# sentence itself rather than a flag, because the five rows do not all ask for the same thing:
# a missing prikk is installed, an old one is updated, and a prikk from another major is answered
# by a newer *stikk*.
$prikkNextStep = ''

function Show-PrikkHowToGetIt {
  # **prikk's command, never a copy of prikk's steps.** It serves a missing prikk and one that is
  # too old alike, the shape `install.sh`'s `prikk_how_to_get_it` has. Writing prikk's manual
  # steps out here would be a second copy of prikk's install design, going stale the day prikk
  # changes it.
  Say 'prikk installs itself. Run this:'
  Say "  $PrikkInstallCmd"
  Say "prikk's install guide: $PrikkInstallGuide"
}

$prikkCmd = Get-Command prikk -ErrorAction SilentlyContinue
if ($prikkCmd) {
  $installed = (& prikk --version 2>$null | Out-String).Trim()
  Say "prikk is already installed: $installed"
  $floorMinor   = [int] $PrikkFloorMinor
  $ceilingMinor = [int] ($PrikkTestedCeiling -split '\.')[1]
  if ($installed -match '(\d+)\.(\d+)\.(\d+)') {
    $major = [int] $Matches[1]
    $minor = [int] $Matches[2]
    if ($major -ne 0) {
      # **A prikk from another major is not "too old"** (review v1 C3). stikk supports major 0
      # only (`Version::is_supported`), so the fix is a newer *stikk*: pointing at prikk would
      # leave the reader on the prikk they already have. Measured: stikk opens such a repository
      # **read-only** rather than refusing.
      # **"may", not "will"** (review v2 C5): on the day prikk $major.0 is published, no stikk
      # supporting it has shipped yet, and promising one would send the reader to reinstall the
      # stikk they already have.
      $prikkNextStep = 'check for a newer stikk (see above)'
      Say "this stikk was built for prikk 0.x and does not know prikk $major.x. It will open your"
      Say 'repository read-only and change nothing. A newer stikk may support it -- check:'
      Say "  $StikkReleases"
    } elseif ($minor -lt $floorMinor) {
      # **Below the floor is the one 0.x row that is not "yours still runs".** Measured: stikk
      # opens read-only here too, so that is what it says rather than leaving it to be found.
      $prikkNextStep = 'install or update prikk (the command is above)'
      Say "that is too old for this stikk, which needs prikk 0.$floorMinor or newer. stikk will open your"
      Say 'repository read-only until you update prikk.'
      Show-PrikkHowToGetIt
    } elseif ($minor -le $ceilingMinor) {
      Say "that is inside the range this stikk was tested against (0.$floorMinor through $PrikkTestedCeiling)"
    } else {
      # **Calm on purpose.** This is the normal state for a few days after every prikk release,
      # and a newcomer must not read it as a fault.
      Say "that is newer than this stikk was tested against (0.$floorMinor through $PrikkTestedCeiling)."
      Say 'It still runs; stikk will say its output has not been checked, rather than assume it has.'
    }
  } else {
    # **Measured** (review v1 C2): stikk's handshake parses this line, and a version it cannot
    # read makes stikk exit with *could not find a version in prikk output*. So this row says
    # what happens, and must not borrow the reassuring sentence the two in-range rows use.
    $prikkNextStep = 'install or update prikk (the command is above)'
    Say 'stikk could not read a version number from that, and will refuse to start with this prikk.'
    Show-PrikkHowToGetIt
  }
} else {
  $prikkNextStep = 'install prikk (the command is above)'
  Say 'stikk reads prikk repositories, so you need prikk too.'
  Show-PrikkHowToGetIt
}

# ---------------------------------------------------------------- what to do next

Write-Host ''
Say 'done. Next:'
# **No "open a new window" step** (decision 3): this session's PATH was updated above, so stikk
# works right here. The steps are numbered as they are actually needed.
$step = 1
if ($prikkNextStep) {
  Say "  $step. $prikkNextStep"
  $step++
}
Say "  $step. make a repository:  prikk init"
$step++
Say "  $step. open it:            stikk"
Say 'Press ? inside stikk for help.'
Write-Host ''
Say 'The checksum above proves this download is the file the release published. It does not prove'
Say 'who built it. For that, with GitHub CLI:'
Say "  gh attestation verify <the archive> -R $Repo"
Say 'To remove stikk later:'
Say "  irm https://github.com/$Repo/releases/latest/download/uninstall.ps1 | iex"
