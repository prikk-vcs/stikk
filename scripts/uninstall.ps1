<#
.SYNOPSIS
  stikk uninstaller for Windows.

.DESCRIPTION
  Removes the stikk.exe the installer put in place, and the entry the installer added to your user
  PATH. It removes nothing else, and never touches the machine-wide PATH.

  It never removes prikk. prikk is a separate program; this script prints where to get rid of it.

.EXAMPLE
  irm https://github.com/prikk-vcs/stikk/releases/latest/download/uninstall.ps1 | iex
#>
# Write-Host is deliberate: this script talks to a person at a console, and `irm | iex` leaves
# no pipeline for Write-Output to go to. Suppressed here rather than per line.
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '',
  Justification = 'This is an interactive installer; its output is for a human, not a pipeline.')]
[CmdletBinding()]
param(
  # Where stikk was installed.
  [string] $Prefix = $env:STIKK_INSTALL_PREFIX
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Repo = 'prikk-vcs/stikk'

function Say { param([string] $Message) Write-Host "stikk-uninstall: $Message" }

if (-not $Prefix) { $Prefix = Join-Path $env:LOCALAPPDATA 'Programs\stikk' }

$removedAnything = $false

$exe = Join-Path $Prefix 'stikk.exe'
if (Test-Path $exe) {
  Remove-Item $exe -Force
  Say "removed $exe"
  $removedAnything = $true
} else {
  Say "no stikk.exe at $exe (nothing to remove there)"
}

# The USER Path only, and only our own entry.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath) {
  $kept = $userPath -split ';' | Where-Object { $_ -and ($_.TrimEnd('\') -ine $Prefix.TrimEnd('\')) }
  $newPath = ($kept -join ';')
  if ($newPath -ne $userPath) {
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    Say "removed $Prefix from your user PATH"
    $removedAnything = $true
  }
}

# Remove the install folder only if it is ours and now empty, so a shared folder is left alone.
if ((Test-Path $Prefix) -and -not (Get-ChildItem $Prefix -Force)) {
  Remove-Item $Prefix -Force
  Say "removed the empty folder $Prefix"
}

if (-not $removedAnything) {
  Say 'nothing of stikk''s was found. If you installed it somewhere else, pass -Prefix DIR.'
}

Write-Host ''
Say "done. stikk's config and state were not touched; 'stikk config path' showed where they live."
Say 'Your repositories are untouched: stikk never wrote to them.'
Write-Host ''
# **prikk is removed by prikk's own uninstaller**, never by this one (RFC 048 decision 2).
# prikk publishes no Windows uninstaller yet -- measured at submission: 0.53.0 is the latest
# release and `uninstall.ps1` is 404 there, and prikk ships one in 0.54.0. So this links prikk's
# guide rather than writing prikk's steps out: they are prikk's to change.
Say 'prikk was NOT removed -- it is a separate program, and prikk uninstalls itself. Its guide'
Say 'has the steps:'
Say '  https://prikk.org/docs/guide/install.html'

Say 'To install stikk again:'
Say "  irm https://github.com/$Repo/releases/latest/download/install.ps1 | iex"
