# Installs the lokey command-line tool for the current user.
#
#   irm https://github.com/akshit-bansal11/lokey/releases/latest/download/install.ps1 | iex
#
# Downloads lokey.exe from the latest GitHub release, checks it against the
# release's SHA256SUMS.txt, puts it in %LOCALAPPDATA%\Programs\lokey and adds
# that folder to your user PATH. No administrator rights, nothing else touched.
# Works in Windows PowerShell 5.1 and PowerShell 7.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$release = 'https://github.com/akshit-bansal11/lokey/releases/latest/download'
$dir = Join-Path $env:LOCALAPPDATA 'Programs\lokey'
$exe = Join-Path $dir 'lokey.exe'
$download = Join-Path $dir 'lokey.exe.download'

New-Item -ItemType Directory -Force -Path $dir | Out-Null

Write-Host 'Downloading lokey.exe ...'
Invoke-WebRequest -Uri "$release/lokey.exe" -OutFile $download -UseBasicParsing

# A checksum from the same release catches a corrupted or truncated download.
# For provenance, verify the build attestation instead:
#   gh attestation verify lokey.exe --repo akshit-bansal11/lokey
# Saved to disk: Invoke-WebRequest returns bytes in PowerShell 5.1 but a string
# in PowerShell 7 for GitHub downloads, and a file reads the same in both.
$sumsFile = Join-Path $dir 'SHA256SUMS.txt'
Invoke-WebRequest -Uri "$release/SHA256SUMS.txt" -OutFile $sumsFile -UseBasicParsing
$sums = Get-Content -LiteralPath $sumsFile -Raw
Remove-Item -LiteralPath $sumsFile -Force
$line = $sums -split "`n" | Where-Object { $_ -match '^\s*([0-9a-fA-F]{64})\s+\*?lokey\.exe\s*$' } | Select-Object -First 1
if (-not $line) {
    Remove-Item -LiteralPath $download -Force
    throw 'SHA256SUMS.txt in the release does not list lokey.exe; nothing was installed.'
}
$expected = ($line.Trim() -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
    Remove-Item -LiteralPath $download -Force
    throw "lokey.exe did not match its published checksum; nothing was installed.`n  expected $expected`n  got      $actual"
}

Move-Item -LiteralPath $download -Destination $exe -Force

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$entries = @($userPath -split ';' | Where-Object { $_ })
if ($entries -notcontains $dir) {
    [Environment]::SetEnvironmentVariable('Path', (($entries + $dir) -join ';'), 'User')
    $env:Path = "$env:Path;$dir"
}

Write-Host ''
Write-Host "Installed $exe"
Write-Host 'Open a new terminal, then create your vault with:  lokey init'
Write-Host 'The lokey desktop app, if you use it, shares the same vault.'
