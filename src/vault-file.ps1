# vault-file.ps1 — disk I/O for the vault: paths, ACLs, atomic writes, wiping.
# Knows nothing about keys; it moves opaque JSON around safely.

Set-StrictMode -Version Latest

function Get-SvPath {
    param([Parameter(Mandatory)][ValidateSet('vault', 'backup', 'session', 'lockout', 'vaultdir', 'statedir')][string]$Kind)

    switch ($Kind) {
        'vaultdir' { $script:SV.VaultDir }
        'statedir' { $script:SV.StateDir }
        'vault' { Join-Path $script:SV.VaultDir $script:SV.VaultName }
        'backup' { Join-Path $script:SV.VaultDir $script:SV.BackupName }
        'session' { Join-Path $script:SV.StateDir $script:SV.SessionName }
        'lockout' { Join-Path $script:SV.StateDir $script:SV.LockoutName }
    }
}

# Strip inheritance and grant only the current SID. Using the SID rather than the
# account name survives renamed and non-English accounts.
function Set-SvDirectoryAcl {
    param([Parameter(Mandatory)][string]$Path)

    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    & icacls.exe $Path /inheritance:r /grant:r "*${sid}:(OI)(CI)F" /Q 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-SvWarn "Could not tighten permissions on $Path — check it is not on a FAT/exFAT volume."
    }
}

function Initialize-SvDirectories {
    foreach ($kind in @('vaultdir', 'statedir')) {
        $path = Get-SvPath $kind
        if (-not (Test-Path -LiteralPath $path)) {
            New-Item -ItemType Directory -Path $path -Force | Out-Null
            Set-SvDirectoryAcl $path
        }
    }
}

function Test-SvVaultExists {
    Test-Path -LiteralPath (Get-SvPath vault)
}

function Read-SvVaultFile {
    $path = Get-SvPath vault
    if (-not (Test-Path -LiteralPath $path)) {
        throw "No vault at $path — create one with:  secure-vault --init"
    }
    $vault = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable
    if ($vault.v -ne $script:SV.Format) {
        throw "Vault format v$($vault.v) is not supported by this build (expects v$($script:SV.Format))."
    }
    $vault
}

# Write to a sibling temp file, then File.Replace so the vault is never a
# half-written file — and the previous good copy lands in vault.sv.bak.
function Write-SvVaultFile {
    param([Parameter(Mandatory)][hashtable]$Vault)

    Initialize-SvDirectories
    $path = Get-SvPath vault
    $temp = "$path.$([guid]::NewGuid().ToString('N')).tmp"
    $json = $Vault | ConvertTo-Json -Depth 12

    Set-Content -LiteralPath $temp -Value $json -Encoding utf8 -NoNewline
    if (Test-Path -LiteralPath $path) {
        [System.IO.File]::Replace($temp, $path, (Get-SvPath backup))
    }
    else {
        [System.IO.File]::Move($temp, $path)
    }
}

# ponytail: 3-pass random overwrite. On SSDs and CoW/journalled volumes the
# controller may retain remapped blocks, so this is best-effort, not forensic
# erasure — full-disk encryption is the real guarantee.
function Remove-SvFileSecurely {
    param([Parameter(Mandatory)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) { return }
    try {
        $length = (Get-Item -LiteralPath $Path).Length
        if ($length -gt 0) {
            $stream = [System.IO.File]::Open($Path, 'Open', 'Write', 'None')
            try {
                for ($pass = 0; $pass -lt $script:SV.WipePasses; $pass++) {
                    $stream.Position = 0
                    $noise = New-SvRandomBytes $length
                    $stream.Write($noise, 0, $noise.Length)
                    $stream.Flush($true)
                    Clear-SvBytes $noise
                }
                $stream.SetLength(0)
                $stream.Flush($true)
            }
            finally { $stream.Dispose() }
        }
    }
    catch { Write-SvWarn "Overwrite of $Path failed ($($_.Exception.Message)); deleting anyway." }
    Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
}
