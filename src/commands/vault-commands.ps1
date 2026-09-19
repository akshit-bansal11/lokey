# vault-commands.ps1 — creating, rotating and destroying the vault itself.

Set-StrictMode -Version Latest

function Assert-SvDistinctPasswords {
    param([Parameter(Mandatory)][hashtable]$Passwords)

    $names = @($Passwords.Keys)
    for ($i = 0; $i -lt $names.Count; $i++) {
        for ($j = $i + 1; $j -lt $names.Count; $j++) {
            if (Test-SvBytesEqual $Passwords[$names[$i]] $Passwords[$names[$j]]) {
                throw "The $($names[$i]) and $($names[$j]) passwords must be different."
            }
        }
    }
}

function Invoke-SvInit {
    if (Test-SvVaultExists) {
        throw "A vault already exists at $(Get-SvPath vault). Use --rotate to change passwords, or --emergency to destroy it."
    }

    Write-SvBanner -State 'NEW VAULT' -Detail (Get-SvPath vault)
    Write-SvInfo 'Three passwords, three jobs:'
    Write-SvDim 'master    opens the file and lists what is inside'
    Write-SvDim 'decrypt   reveals the values themselves'
    Write-SvDim 'deletion  destroys the whole vault via --emergency'
    Write-SvWarn "There is no recovery. Lose the master or decrypt password and the data is gone."
    Write-Host ''

    $passwords = @{}
    try {
        $passwords['master'] = Read-SvPassword -Prompt 'master password' -Confirm -EnforceLength
        $passwords['decrypt'] = Read-SvPassword -Prompt 'decrypt password' -Confirm -EnforceLength
        $passwords['deletion'] = Read-SvPassword -Prompt 'deletion password' -Confirm -EnforceLength
        Assert-SvDistinctPasswords -Passwords $passwords

        Write-SvDim "deriving keys ($($script:SV.Iterations) pbkdf2 iterations)..."
        $vault = New-SvVaultHeader -ErasePassword $passwords['deletion']
        $masterKey = Get-SvMasterKey -Password $passwords['master'] -Vault $vault
        $dataKey = Get-SvDataKey -Password $passwords['decrypt'] -Vault $vault
        try {
            $body = New-SvBody
            Set-SvBodyCheck -Body $body -DataKey $dataKey
            Save-SvVault -Vault $vault -MasterKey $masterKey -Body $body
            Clear-SvFailures
            Clear-SvSession
        }
        finally {
            Clear-SvBytes $masterKey
            Clear-SvBytes $dataKey
        }

        Write-SvOk "created $(Get-SvPath vault)"
        Write-SvDim 'next:  secure-vault --unlock   then   secure-vault --add'
    }
    finally { foreach ($password in $passwords.Values) { Clear-SvBytes $password } }
}

# Fresh salts and a fresh nonce for every value. Passwords may be kept by
# pressing Enter, in which case rotation still re-randomises the whole file.
function Invoke-SvRotate {
    $context = Open-SvContext -PromptMaster
    try {
        $dataKey = Get-SvUnlockedDataKey -Context $context
        if ($null -eq $dataKey) { $dataKey = Read-SvDataKeyInteractive -Context $context }

        $entries = @(Get-SvEntries -Body $context.Body)
        $plaintexts = Get-SvDecryptedValues -Body $context.Body -DataKey $dataKey -Entries $entries
        Clear-SvBytes $dataKey

        Write-SvBanner -State 'ROTATE' -Detail "$($entries.Count) entries · new salts and nonces"
        Write-SvDim 'press Enter at any prompt to keep that password unchanged'

        $passwords = @{}
        try {
            $passwords['master'] = Read-SvPassword -Prompt 'new master password' -Confirm -EnforceLength -AllowEmpty
            $passwords['decrypt'] = Read-SvPassword -Prompt 'new decrypt password' -Confirm -EnforceLength -AllowEmpty
            $passwords['deletion'] = Read-SvPassword -Prompt 'new deletion password' -Confirm -EnforceLength -AllowEmpty

            $kept = @($passwords.Keys | Where-Object { $null -eq $passwords[$_] })
            if ($kept.Count -gt 0) {
                Write-SvDim "re-enter the current password for: $($kept -join ', ')"
                foreach ($name in $kept) {
                    $passwords[$name] = Read-SvPassword -Prompt "current $name password"
                }
            }
            Assert-SvDistinctPasswords -Passwords $passwords

            Write-SvDim 'deriving keys...'
            $newVault = New-SvVaultHeader -ErasePassword $passwords['deletion']
            $newVault.meta.created = $context.Vault.meta.created
            $newMasterKey = Get-SvMasterKey -Password $passwords['master'] -Vault $newVault
            $newDataKey = Get-SvDataKey -Password $passwords['decrypt'] -Vault $newVault

            try {
                $body = New-SvBody
                $body.projects = @($context.Body.projects)
                $body.current = $context.Body.current
                Set-SvBodyCheck -Body $body -DataKey $newDataKey
                foreach ($entry in $entries) {
                    $entry.val = Protect-SvValue -DataKey $newDataKey -Entry $entry -Plaintext $plaintexts[$entry.id]
                    $body.entries = @($body.entries) + $entry
                }
                Save-SvVault -Vault $newVault -MasterKey $newMasterKey -Body $body
            }
            finally {
                Clear-SvBytes $newMasterKey
                Clear-SvBytes $newDataKey
            }

            Clear-SvSession
            Clear-SvFailures
            Write-SvOk "rotated $($entries.Count) entries"

            # The backup still opens with the OLD passwords, so leaving it around
            # undoes the rotation for anyone holding them. Offer to destroy it now.
            Write-SvWarn "$($script:SV.BackupName) still opens with the OLD passwords."
            if ((Read-SvLine -Prompt 'wipe that old backup now? [Y/n]') -inotmatch '^n') {
                Remove-SvFileSecurely (Get-SvPath backup)
                Write-SvOk 'old backup wiped'
            }
            else {
                Write-SvDim "left at $(Get-SvPath backup) — delete it yourself once the new passwords are verified"
            }
        }
        finally { foreach ($password in $passwords.Values) { Clear-SvBytes $password } }
    }
    finally { Clear-SvBytes $context.MasterKey }
}

# Needs the deletion password only — no master, no decrypt. The verifier for it
# sits in the cleartext header precisely so this works when nothing else does.
function Invoke-SvEmergency {
    param([switch]$Force)

    Assert-SvNotLockedOut
    $vault = Read-SvVaultFile

    Write-SvBanner -State 'EMERGENCY' -Detail 'irreversible'
    Write-SvError "This destroys $(Get-SvPath vault), its backup, and every cached key."

    $password = Read-SvPassword -Prompt 'deletion password'
    try {
        if (-not (Test-SvErasePassword -Password $password -Vault $vault)) {
            Add-SvFailure -What 'deletion password'
            return
        }
    }
    finally { Clear-SvBytes $password }
    Clear-SvFailures

    if (-not $Force) {
        if (-not (Confirm-SvPhrase -Phrase 'ERASE' -Prompt "type ERASE to confirm (or --force to skip this)")) {
            Write-SvDim 'cancelled — nothing was touched'
            return
        }
    }

    foreach ($kind in @('vault', 'backup', 'session', 'lockout')) {
        Remove-SvFileSecurely (Get-SvPath $kind)
    }
    foreach ($kind in @('vaultdir', 'statedir')) {
        $path = Get-SvPath $kind
        if ((Test-Path -LiteralPath $path) -and -not (Get-ChildItem -LiteralPath $path -Force)) {
            Remove-Item -LiteralPath $path -Force -Recurse -ErrorAction SilentlyContinue
        }
    }

    Write-SvOk 'Vault erased.'
    Write-SvDim 'note: on SSDs, overwritten blocks may survive in unmapped flash. Full-disk encryption is the real guarantee.'
}
