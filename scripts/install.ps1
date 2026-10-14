<#
.SYNOPSIS
  stikk installer for Windows.

.DESCRIPTION
  Downloads the stikk release this script was published with, checks it against the checksum
  published beside it, and installs it into a directory you own. It installs nothing if that check
  fails.

  prikk -- the version control system stikk reads -- has no Windows installer of its own, so this
  script does not improvise one. If prikk is missing it says exactly how to get it.

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
  [string] $BaseUrl   = $env:STIKK_INSTALL_BASE_URL,
  # Accepted so the same options work on every platform. prikk has no Windows installer, so
  # -WithPrikk cannot install it here -- it says so rather than failing silently.
  [switch] $WithPrikk,
  [switch] $NoPrikk
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Filled by release.yml at release time. @-delimited so a half-substituted script is obvious.
$StikkVersion       = '@STIKK_VERSION@'
$PrikkPinnedVersion = '@PRIKK_VERSION@'

$Repo      = 'prikk-vcs/stikk'
$PrikkRepo = 'prikk-vcs/prikk'

function Say  { param([string] $Message) Write-Host "stikk-install: $Message" }
function Die  { param([string] $Message) Write-Host "stikk-install: $Message" -ForegroundColor Red; exit 1 }

if (-not $Version) { $Version = $StikkVersion }
if (-not $Prefix)  { $Prefix  = Join-Path $env:LOCALAPPDATA 'Programs\stikk' }

# The placeholders above are replaced at release time. Detect an unfilled one by its shape rather
# than by comparing against the literal -- the substitution would rewrite the comparison too, and
# the guard would then fire on every real install.
if (($Version + $PrikkPinnedVersion) -like '*@*') {
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
$pathChanged = $false
if (-not $already) {
  $newPath = if ($userPath.TrimEnd(';')) { $userPath.TrimEnd(';') + ';' + $Prefix } else { $Prefix }
  [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
  $env:Path = "$env:Path;$Prefix"
  Say "added $Prefix to your user PATH"
  $pathChanged = $true
} else {
  Say "$Prefix is already on your user PATH"
}

# ---------------------------------------------------------------- prikk

$prikkCmd = Get-Command prikk -ErrorAction SilentlyContinue
if ($prikkCmd) {
  $installed = (& prikk --version 2>$null | Out-String).Trim()
  Say "prikk is already installed: $installed"
  if ($installed -match '(\d+)\.(\d+)\.(\d+)') {
    $minor       = [int] $Matches[2]
    $pinnedMinor = [int] ($PrikkPinnedVersion -split '\.')[1]
    if ($minor -ge 28 -and $minor -le $pinnedMinor) {
      Say "that is inside the range this stikk was tested against (0.28 through $PrikkPinnedVersion)"
    } else {
      Say "this stikk was tested against prikk 0.28 through $PrikkPinnedVersion. Yours still runs;"
      Say 'stikk will say its output has not been checked, rather than assume it has.'
    }
  }
} elseif (-not $NoPrikk) {
  # prikk publishes no Windows installer, so there is nothing honest to run here.
  Say 'stikk reads prikk repositories, so you need prikk too.'
  if ($WithPrikk) {
    # The flag was given and cannot be honoured. Say that, rather than appearing to have acted.
    Say 'you asked for prikk to be installed, but prikk publishes no Windows installer, and stikk'
    Say 'will not improvise one. Install it one of these ways:'
  } else {
    Say 'prikk has no Windows installer, so stikk will not improvise one. Install it one of these ways:'
  }
  Say "  1. download prikk-x86_64-pc-windows-msvc.zip from"
  Say "     https://github.com/$PrikkRepo/releases/tag/$PrikkPinnedVersion"
  Say '     and put prikk.exe in a folder on your PATH'
  Say "  2. cargo install prikk --version $PrikkPinnedVersion   (needs Rust, and compiles)"
}

# ---------------------------------------------------------------- what to do next

Write-Host ''
Say 'done. Next:'
if ($pathChanged) {
  Say '  1. open a new PowerShell window (PATH changed)'
  Say '  2. make a repository:  prikk init'
  Say '  3. open it:            stikk'
} else {
  Say '  1. make a repository:  prikk init'
  Say '  2. open it:            stikk'
}
Say 'Press ? inside stikk for help.'
Write-Host ''
Say 'The checksum above proves this download is the file the release published. It does not prove'
Say 'who built it. For that, with GitHub CLI:'
Say "  gh attestation verify <the archive> -R $Repo"
Say 'To remove stikk later:'
Say "  irm https://github.com/$Repo/releases/latest/download/uninstall.ps1 | iex"
