# session.ps1 — why `--lock` and `--unlock` can be separate commands.
#
# A derived key is cached between invocations wrapped in Windows DPAPI at
# CurrentUser scope, with the vault's own master salt mixed in as entropy. Only
# this Windows account on this machine can unwrap it, and rotating the vault
# invalidates every cached key automatically. Plaintext secrets are never cached.

Set-StrictMode -Version Latest

function Get-SvSessionEntropy {
    param([Parameter(Mandatory)][string]$Kind, [Parameter(Mandatory)]$Vault)

    , [System.Text.Encoding]::UTF8.GetBytes("$($script:SV.Name)|$Kind|$($Vault.kdf.sm)")
}

function Get-SvSessionTtlMinutes {
    param([Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind)

    if ($Kind -eq 'master') { $script:SV.OpenTtlMinutes } else { $script:SV.UnlockTtlMinutes }
}

# Deadlines are stored as Unix seconds, never as formatted dates.
# ConvertFrom-Json -AsHashtable rehydrates ISO-8601 strings into [datetime], and
# re-parsing those depends on the current culture — under en-IN, an ISO date
# round-trips to an unparseable "08/17/2026". Integers have no such opinion.
function Get-SvDeadline {
    param([Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind)

    [DateTimeOffset]::UtcNow.AddMinutes((Get-SvSessionTtlMinutes $Kind)).ToUnixTimeSeconds()
}

function Test-SvDeadlinePassed {
    param([Parameter(Mandatory)][long]$Deadline)

    [DateTimeOffset]::UtcNow.ToUnixTimeSeconds() -gt $Deadline
}

function Read-SvSessionFile {
    $path = Get-SvPath session
    if (-not (Test-Path -LiteralPath $path)) { return @{} }
    try { Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable }
    catch { @{} }
}

function Write-SvSessionFile {
    param([Parameter(Mandatory)][hashtable]$Session)

    Initialize-SvDirectories
    Set-Content -LiteralPath (Get-SvPath session) -Value ($Session | ConvertTo-Json -Depth 6) -Encoding utf8 -NoNewline
}

function Set-SvCachedKey {
    param(
        [Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind,
        [Parameter(Mandatory)][byte[]]$Key,
        [Parameter(Mandatory)]$Vault
    )

    $entropy = Get-SvSessionEntropy -Kind $Kind -Vault $Vault
    $blob = [System.Security.Cryptography.ProtectedData]::Protect(
        $Key, $entropy, [System.Security.Cryptography.DataProtectionScope]::CurrentUser)

    $session = Read-SvSessionFile
    $session.bind = $Vault.kdf.sm
    $session[$Kind] = @{
        blob    = [Convert]::ToBase64String($blob)
        expires = Get-SvDeadline -Kind $Kind
    }
    Write-SvSessionFile -Session $session
}

# Returns the key and slides the expiry forward, so the TTL is an idle timeout.
function Get-SvCachedKey {
    param(
        [Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind,
        [Parameter(Mandatory)]$Vault
    )

    $session = Read-SvSessionFile
    if ($session.Count -eq 0 -or $null -eq $session[$Kind]) { return $null }
    if ($session.bind -ne $Vault.kdf.sm) { Clear-SvSession; return $null }
    if (Test-SvDeadlinePassed -Deadline ([long]$session[$Kind].expires)) {
        Remove-SvCachedKey -Kind $Kind
        return $null
    }

    try {
        $entropy = Get-SvSessionEntropy -Kind $Kind -Vault $Vault
        $key = [System.Security.Cryptography.ProtectedData]::Unprotect(
            [Convert]::FromBase64String($session[$Kind].blob),
            $entropy,
            [System.Security.Cryptography.DataProtectionScope]::CurrentUser)
    }
    catch { Remove-SvCachedKey $Kind; return $null }

    $session[$Kind].expires = Get-SvDeadline -Kind $Kind
    Write-SvSessionFile -Session $session
    , $key
}

function Get-SvCachedKeyExpiry {
    param(
        [Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind,
        [Parameter(Mandatory)]$Vault
    )

    $session = Read-SvSessionFile
    if ($session.Count -eq 0 -or $null -eq $session[$Kind]) { return $null }
    if ($session.bind -ne $Vault.kdf.sm) { return $null }

    $expires = [long]$session[$Kind].expires
    if (Test-SvDeadlinePassed -Deadline $expires) { return $null }
    [DateTimeOffset]::FromUnixTimeSeconds($expires).LocalDateTime
}

function Remove-SvCachedKey {
    param([Parameter(Mandatory)][ValidateSet('master', 'data')][string]$Kind)

    $session = Read-SvSessionFile
    if ($session.Count -eq 0) { return }
    $session.Remove($Kind)
    if ($null -eq $session['master'] -and $null -eq $session['data']) { Clear-SvSession }
    else { Write-SvSessionFile -Session $session }
}

function Clear-SvSession {
    Remove-SvFileSecurely (Get-SvPath session)
}
