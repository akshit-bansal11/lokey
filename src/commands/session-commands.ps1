# session-commands.ps1 — opening, unlocking, locking, status.

Set-StrictMode -Version Latest

# Every command that needs the vault goes through here. Returns
# @{ Vault; MasterKey; Body }. Caller is responsible for clearing MasterKey.
function Open-SvContext {
    param([switch]$PromptMaster)

    Assert-SvNotLockedOut
    $vault = Read-SvVaultFile

    if (-not $PromptMaster) {
        $cached = Get-SvCachedKey -Kind master -Vault $vault
        if ($cached) {
            try { return @{ Vault = $vault; MasterKey = $cached; Body = (Open-SvEnvelope -Vault $vault -MasterKey $cached) } }
            catch [System.Security.Cryptography.CryptographicException] {
                Clear-SvBytes $cached
                Remove-SvCachedKey -Kind master
            }
        }
    }

    while ($true) {
        $password = Read-SvPassword -Prompt 'master password'
        try { $key = Get-SvMasterKey -Password $password -Vault $vault }
        finally { Clear-SvBytes $password }

        try {
            $body = Open-SvEnvelope -Vault $vault -MasterKey $key
            Clear-SvFailures
            Set-SvCachedKey -Kind master -Key $key -Vault $vault
            return @{ Vault = $vault; MasterKey = $key; Body = $body }
        }
        catch [System.Security.Cryptography.CryptographicException] {
            Clear-SvBytes $key
            Add-SvFailure -What 'master password'
        }
    }
}

function Get-SvUnlockedDataKey {
    param([Parameter(Mandatory)][hashtable]$Context)

    $key = Get-SvCachedKey -Kind data -Vault $Context.Vault
    if ($key -and (Test-SvDataKey -DataKey $key -Body $Context.Body)) { return , $key }
    if ($key) { Clear-SvBytes $key; Remove-SvCachedKey -Kind data }
    $null
}

function Assert-SvUnlocked {
    param([Parameter(Mandatory)][hashtable]$Context)

    $key = Get-SvUnlockedDataKey -Context $Context
    if ($null -eq $key) { throw 'Vault is locked. Run:  secure-vault --unlock' }
    , $key
}

function Read-SvDataKeyInteractive {
    param([Parameter(Mandatory)][hashtable]$Context)

    while ($true) {
        $password = Read-SvPassword -Prompt 'decrypt password'
        try { $key = Get-SvDataKey -Password $password -Vault $Context.Vault }
        finally { Clear-SvBytes $password }

        if (Test-SvDataKey -DataKey $key -Body $Context.Body) {
            Clear-SvFailures
            Set-SvCachedKey -Kind data -Key $key -Vault $Context.Vault
            return , $key
        }
        Clear-SvBytes $key
        Add-SvFailure -What 'decrypt password'
    }
}

# Returns the selector's action request (move/delete) for the caller to run once
# this function's keys have been cleared, or $null.
function Show-SvUnlockedView {
    param([Parameter(Mandatory)][hashtable]$Context, [Parameter(Mandatory)][byte[]]$DataKey)

    $project = $Context.Body.current
    $entries = @(Get-SvEntries -Body $Context.Body -Project $project)
    $expires = Get-SvCachedKeyExpiry -Kind data -Vault $Context.Vault

    $detail = "$project · $($entries.Count) entries"
    if ($expires) { $detail += " · auto-locks $($expires.ToString('HH:mm:ss'))" }
    Write-SvBanner -State 'UNLOCKED' -Detail $detail

    if ($entries.Count -eq 0) {
        Write-SvDim "No entries in '$project'. Add one with:  secure-vault --add"
        return $null
    }
    # Hand the selector a way to decrypt ONE entry rather than a bag of every
    # plaintext in the project: listing the vault must not put secrets you never
    # look at into the managed heap.
    #
    # The key travels as an argument, never as a captured variable. GetNewClosure
    # would rehost this block in a fresh dynamic-module scope, and that scope
    # cannot see functions dot-sourced into a *script* scope -- which is exactly
    # what the installed `secure-vault` profile wrapper creates. Pressing Enter
    # on a row then died with "Unprotect-SvValue is not recognized".
    $reveal = {
        param($entry, $dataKey)
        Get-SvDisplayValue -Entry $entry -Payload (Unprotect-SvValue -DataKey $dataKey -Entry $entry)
    }

    Show-SvRevealTable -Entries $entries -Reveal $reveal -DataKey $DataKey
}

# M and D in the selector land here, after the interactive loop has torn down
# and the caller's keys are cleared. Both keys are still cached, so neither
# command re-prompts for a password.
function Invoke-SvSelectorAction {
    param($Action)

    if ($null -eq $Action) { return }
    switch ($Action.Action) {
        'move' { Invoke-SvMove -Name $Action.Name }
        'delete' { Invoke-SvRemove -Name $Action.Name }
    }
}

# Default command: always asks for the master password, as specified.
function Invoke-SvOpen {
    $context = Open-SvContext -PromptMaster
    $action = $null
    try {
        $dataKey = Get-SvUnlockedDataKey -Context $context
        if ($dataKey) {
            try { $action = Show-SvUnlockedView -Context $context -DataKey $dataKey }
            finally { Clear-SvBytes $dataKey }
        }
        else {
            $project = $context.Body.current
            $entries = @(Get-SvEntries -Body $context.Body -Project $project)
            Write-SvBanner -State 'OPEN' -Detail "$project · $($entries.Count) entries · values still encrypted"
            Show-SvCipherTable -Entries $entries
        }
    }
    finally { Clear-SvBytes $context.MasterKey }

    Invoke-SvSelectorAction -Action $action
}

function Invoke-SvUnlock {
    $context = Open-SvContext
    $action = $null
    try {
        $dataKey = Get-SvUnlockedDataKey -Context $context
        if ($null -eq $dataKey) { $dataKey = Read-SvDataKeyInteractive -Context $context }
        try { $action = Show-SvUnlockedView -Context $context -DataKey $dataKey }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }

    Invoke-SvSelectorAction -Action $action
}

function Invoke-SvLock {
    Clear-SvSession
    Write-SvBanner -State 'LOCKED' -Detail 'both cached keys forgotten'
}

# Deliberately prompts for nothing at all.
function Invoke-SvStatus {
    if (-not (Test-SvVaultExists)) {
        Write-SvBanner -State 'NO VAULT' -Detail (Get-SvPath vault)
        Write-SvDim 'create one with:  secure-vault --init'
        return
    }

    $vault = Read-SvVaultFile
    $masterExpiry = Get-SvCachedKeyExpiry -Kind master -Vault $vault
    $dataExpiry = Get-SvCachedKeyExpiry -Kind data -Vault $vault
    $state = if ($dataExpiry) { 'UNLOCKED' } elseif ($masterExpiry) { 'OPEN' } else { 'LOCKED' }

    Write-SvBanner -State $state
    Write-SvInfo "vault      $(Get-SvPath vault)  ($((Get-Item -LiteralPath (Get-SvPath vault)).Length) B)"
    Write-SvInfo "cipher     aes-256-gcm · $($vault.kdf.alg) · $($vault.kdf.iter) iterations"
    Write-SvInfo "created    $($vault.meta.created)"
    Write-SvInfo "rotated    $($vault.meta.rotated)"
    Write-SvInfo "master key $(if ($masterExpiry) { "cached until $($masterExpiry.ToString('HH:mm:ss'))" } else { 'not cached' })"
    Write-SvInfo "data key   $(if ($dataExpiry) { "cached until $($dataExpiry.ToString('HH:mm:ss'))" } else { 'not cached' })"

    $lockout = Get-SvLockoutRemaining
    if ($lockout) { Write-SvError ('lockout    {0:n0}m {1:n0}s remaining' -f [Math]::Floor($lockout.TotalMinutes), $lockout.Seconds) }
    else { Write-SvInfo "lockout    none · $((Read-SvLockout).fails) recent failure(s)" }
    Write-Host ''
}
