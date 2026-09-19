#Requires -Version 7.0
# install.ps1 — puts a `secure-vault` command in your PowerShell 7 profile.
# Run with -Remove to take it back out. Re-running replaces the old block
# instead of stacking another copy.

[CmdletBinding()]
param([switch]$Remove)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$begin = '# >>> secure-vault >>>'
$end = '# <<< secure-vault <<<'
$entry = Join-Path $PSScriptRoot 'secure-vault.ps1'

if (-not (Test-Path -LiteralPath $entry)) { throw "Cannot find $entry" }

if (-not (Test-Path -LiteralPath $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force | Out-Null
}

# Drop any previous block, then re-add unless we are removing.
$kept = @(Get-Content -LiteralPath $PROFILE)
$out = [System.Collections.Generic.List[string]]::new()
$inside = $false
foreach ($line in $kept) {
    if ($line -eq $begin) { $inside = $true; continue }
    if ($line -eq $end) { $inside = $false; continue }
    if (-not $inside) { $out.Add($line) }
}

if (-not $Remove) {
    $out.Add($begin)
    $out.Add("function secure-vault { & '$entry' @args }")
    $out.Add($end)
}

Set-Content -LiteralPath $PROFILE -Value $out -Encoding utf8

Write-Host ''
if ($Remove) {
    Write-Host "  removed the secure-vault function from $PROFILE" -ForegroundColor Yellow
}
else {
    Write-Host '  installed' -ForegroundColor Green
    Write-Host "  profile   $PROFILE"
    Write-Host "  script    $entry"
    Write-Host "  vault     $(Join-Path ([Environment]::GetFolderPath('UserProfile')) '.secure-vault')"
    Write-Host ''
    Write-Host '  open a new PowerShell 7 window, then:' -ForegroundColor Cyan
    Write-Host '    secure-vault --init'
    Write-Host ''
    Write-Host '  if you use more than one pwsh install, run this script from each of them —' -ForegroundColor DarkGray
    Write-Host '  the profile path above is per-install. The vault itself is shared.' -ForegroundColor DarkGray
}
Write-Host ''
