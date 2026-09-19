# lockout.ps1 — exponential backoff on every wrong password, then a hard lockout
# that survives closing PowerShell.
#
# ponytail: the counter is a plain file, so someone with write access to it can
# reset it. The real brute-force cost is the 600k-iteration KDF; this only stops
# a human or script hammering the prompt.

Set-StrictMode -Version Latest

function Get-SvBackoffSeconds {
    param([Parameter(Mandatory)][int]$Failures)

    if ($Failures -le 0) { return 0 }
    [Math]::Min($script:SV.BackoffCapSec, [Math]::Pow(2, [Math]::Min(30, $Failures - 1)))
}

function Read-SvLockout {
    $path = Get-SvPath lockout
    if (-not (Test-Path -LiteralPath $path)) { return @{ fails = 0; until = $null } }
    try {
        $state = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json -AsHashtable
        @{ fails = [int]$state.fails; until = $state.until }
    }
    catch { @{ fails = 0; until = $null } }
}

function Write-SvLockout {
    param([Parameter(Mandatory)][hashtable]$State)

    Initialize-SvDirectories
    Set-Content -LiteralPath (Get-SvPath lockout) -Value ($State | ConvertTo-Json) -Encoding utf8 -NoNewline
}

# Unix seconds, for the same culture-independence reason as the session file.
function Get-SvLockoutRemaining {
    $state = Read-SvLockout
    if (-not $state.until) { return $null }

    $remaining = [DateTimeOffset]::FromUnixTimeSeconds([long]$state.until) - [DateTimeOffset]::UtcNow
    if ($remaining.TotalSeconds -le 0) { return $null }
    $remaining
}

function Assert-SvNotLockedOut {
    $remaining = Get-SvLockoutRemaining
    if ($remaining) {
        throw ('Locked out after {0} wrong passwords. Try again in {1:n0}m {2:n0}s.' -f `
                $script:SV.MaxFails, [Math]::Floor($remaining.TotalMinutes), $remaining.Seconds)
    }
}

function Add-SvFailure {
    param([string]$What = 'password')

    $state = Read-SvLockout
    $state.fails = $state.fails + 1
    $state.until = $null

    if ($state.fails -ge $script:SV.MaxFails) {
        $state.until = [DateTimeOffset]::UtcNow.AddMinutes($script:SV.LockoutMinutes).ToUnixTimeSeconds()
        $state.fails = 0
        Write-SvLockout -State $state
        Clear-SvSession
        throw "Wrong $What. That was attempt $($script:SV.MaxFails) — vault locked for $($script:SV.LockoutMinutes) minutes."
    }

    Write-SvLockout -State $state
    $wait = Get-SvBackoffSeconds -Failures $state.fails
    $left = $script:SV.MaxFails - $state.fails
    Write-SvError "Wrong $What. $left attempt(s) before a $($script:SV.LockoutMinutes) minute lockout."
    if ($wait -gt 0) {
        Write-SvDim "waiting ${wait}s..."
        Start-Sleep -Seconds $wait
    }
}

function Clear-SvFailures {
    $path = Get-SvPath lockout
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue }
}
