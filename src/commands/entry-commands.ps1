# entry-commands.ps1 — reading, adding, moving and deleting entries.
#
# There is no edit: replacing a secret is delete-then-add, so a value is never
# quietly swapped underneath a name you have already trusted. Moving and
# deleting need only the master password; anything that reads or writes a value
# needs the vault unlocked.
#
# Lookups are scoped to the active project first, then fall back to the whole
# vault, so --show NAME keeps working without a --project dance.

Set-StrictMode -Version Latest

function Read-SvSecretBytes {
    param([Parameter(Mandatory)][string]$Type)

    if ($Type -ne 'codes') {
        $label = if ($Type -eq 'pw') { 'password' } else { 'value' }
        return , (Read-SvPassword -Prompt $label -Confirm)
    }

    Write-SvDim 'paste one code per line, blank line to finish'
    $parts = Read-SvSecretLines -Prompt 'code'
    if ($parts.Count -eq 0) { throw 'No codes entered.' }

    try { , (Join-SvBytes -Parts $parts) }
    finally { foreach ($part in $parts) { Clear-SvBytes $part } }
}

function Find-SvEntryInContext {
    param([Parameter(Mandatory)][hashtable]$Context, [Parameter(Mandatory)][string]$Name)

    Find-SvEntry -Body $Context.Body -Name $Name -Project $Context.Body.current
}

function Invoke-SvShow {
    param([string]$Name)

    if (-not $Name) { throw 'Usage:  secure-vault --show NAME' }

    $context = Open-SvContext
    try {
        $dataKey = Assert-SvUnlocked -Context $context
        try {
            $entry = Find-SvEntryInContext -Context $context -Name $Name
            $payload = Unprotect-SvValue -DataKey $dataKey -Entry $entry

            Write-SvBanner -State 'UNLOCKED' -Detail "$($entry.project) / $($entry.name) · $($entry.type)"
            if ($entry.note) { Write-SvDim $entry.note }
            if ($entry.type -eq 'codes') { Show-SvCodes -Entry $entry -Codes (Split-SvCodes $payload) }
            else { Write-Host "  $payload"; Write-Host '' }
        }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }
}

function Invoke-SvShowAll {
    $context = Open-SvContext
    try {
        $dataKey = Assert-SvUnlocked -Context $context
        try {
            $project = $context.Body.current
            $entries = @(Get-SvEntries -Body $context.Body -Project $project)
            $values = Get-SvDecryptedValues -Body $context.Body -DataKey $dataKey -Entries $entries
            $display = @{}
            foreach ($entry in $entries) { $display[$entry.id] = Get-SvDisplayValue -Entry $entry -Payload $values[$entry.id] }

            Write-SvBanner -State 'UNLOCKED' -Detail "$project · $($entries.Count) entries · nothing masked"
            Show-SvPlainTable -Entries $entries -Values $display
        }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }
}

function Invoke-SvAdd {
    $context = Open-SvContext
    try {
        $dataKey = Assert-SvUnlocked -Context $context
        try {
            $project = $context.Body.current
            Write-SvBanner -State 'UNLOCKED' -Detail "new entry in $project"
            foreach ($type in $script:SV.Types) { Write-SvDim "$type`t$($script:SV.TypeHelp[$type])" }

            $type = Read-SvLine -Prompt "type [$($script:SV.Types -join '|')]" -Default 'kv'
            if ($script:SV.Types -notcontains $type) { throw "Unknown type '$type'." }

            $name = Read-SvLine -Prompt 'name'
            if (-not $name) { throw 'A name is required.' }
            $note = Read-SvLine -Prompt 'note (optional)'

            $entry = New-SvEntry -Type $type -Name $name -Project $project -Note $note
            Add-SvEntry -Body $context.Body -Entry $entry

            $secret = Read-SvSecretBytes -Type $type
            try {
                # Counted at the byte level so the codes never become a string here.
                if ($type -eq 'codes') { $entry.total = 1 + @($secret | Where-Object { $_ -eq 0x0A }).Count }
                $entry.val = Protect-SvValueBytes -DataKey $dataKey -Entry $entry -Plaintext $secret
            }
            finally { Clear-SvBytes $secret }

            Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body
            Write-SvOk "saved $($entry.name) ($type) to $project"
        }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }
}

# Bulk paste:  key1,value1; key2,value2;  in any mix of newlines and spaces.
# Parsing needs the pasted text as a string, same as decrypting a value for
# display does — the bytes are zeroed either side of that.
function Invoke-SvImport {
    $context = Open-SvContext
    try {
        $dataKey = Assert-SvUnlocked -Context $context
        try {
            $project = $context.Body.current
            Write-SvBanner -State 'UNLOCKED' -Detail "bulk import into $project"
            Write-SvDim 'paste  key,value;  pairs — blank line to finish'
            Write-SvDim 'example:   DATABASE_URL,postgres://user:pw@host/db;'

            $lines = Read-SvSecretLines -Prompt 'line'
            if ($lines.Count -eq 0) { throw 'Nothing pasted.' }
            $joined = Join-SvBytes -Parts $lines
            try { $text = [System.Text.Encoding]::UTF8.GetString($joined) }
            finally {
                Clear-SvBytes $joined
                foreach ($line in $lines) { Clear-SvBytes $line }
            }

            $parsed = ConvertFrom-SvPairText -Text $text

            # Refuse the whole batch on a malformed line rather than importing
            # half of it and leaving you to work out which half.
            if (@($parsed.bad).Count -gt 0) {
                Write-SvError "$(@($parsed.bad).Count) line(s) are not  key,value  pairs — nothing was imported:"
                foreach ($line in $parsed.bad) { Write-SvDim "  $($line.Substring(0, [Math]::Min(40, $line.Length)))" }
                return
            }
            if (@($parsed.pairs).Count -eq 0) { throw 'No key,value pairs found.' }

            $added = 0
            $skipped = @()
            foreach ($pair in $parsed.pairs) {
                $entry = New-SvEntry -Type 'kv' -Name $pair.name -Project $project
                try { Add-SvEntry -Body $context.Body -Entry $entry }
                catch { $skipped += $pair.name; continue }

                $entry.val = Protect-SvValue -DataKey $dataKey -Entry $entry -Plaintext $pair.value
                $added++
                Write-SvDim "  + $($pair.name)"
            }

            if ($added -gt 0) { Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body }
            Write-SvOk "$added imported into $project"
            if ($skipped.Count -gt 0) {
                Write-SvWarn "skipped, already in $project`: $($skipped -join ', ')"
                Write-SvDim 'to replace one:  secure-vault --rm NAME   then add it again'
            }
        }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }
}

function Read-SvProjectChoice {
    param([Parameter(Mandatory)][hashtable]$Body, [string]$Exclude)

    $choices = @($Body.projects | Where-Object { $_ -ne $Exclude })
    if ($choices.Count -eq 0) {
        throw 'There is no other project to move into. Create one with:  secure-vault --project NAME'
    }

    Show-SvProjectChoices -Choices $choices
    $answer = Read-SvLine -Prompt "move to [1-$($choices.Count) or name]"
    if (-not $answer) { return $null }

    $index = 0
    if ([int]::TryParse($answer, [ref]$index)) {
        if ($index -lt 1 -or $index -gt $choices.Count) { throw "There is no project number $index." }
        return $choices[$index - 1]
    }

    $match = @($choices | Where-Object { $_ -ieq $answer })
    if ($match.Count -ne 1) { throw "No project named '$answer'." }
    $match[0]
}

# Moving needs only the master password: the ciphertext is untouched, just
# re-filed. See Move-SvEntry for why no re-encryption is involved.
function Invoke-SvMove {
    param([string]$Name)

    if (-not $Name) { throw 'Usage:  secure-vault --move NAME' }

    $context = Open-SvContext
    try {
        $entry = Find-SvEntryInContext -Context $context -Name $Name
        Write-SvBanner -State 'OPEN' -Detail "move $($entry.project) / $($entry.name) · $($entry.type)"

        $target = Read-SvProjectChoice -Body $context.Body -Exclude $entry.project
        if (-not $target) {
            Write-SvDim 'cancelled'
            return
        }

        $moved = Move-SvEntry -Body $context.Body -Entry $entry -ToProject $target
        Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body
        Write-SvOk "moved $($entry.name) to $moved"
    }
    finally { Clear-SvBytes $context.MasterKey }
}

# Deleting needs only the master password: the name is enough to identify what
# goes, and requiring the decrypt password would not make the deletion safer.
function Invoke-SvRemove {
    param([string]$Name)

    if (-not $Name) { throw 'Usage:  secure-vault --rm NAME' }

    $context = Open-SvContext
    try {
        $entry = Find-SvEntryInContext -Context $context -Name $Name
        Write-SvBanner -State 'OPEN' -Detail "delete $($entry.project) / $($entry.name) · $($entry.type)"
        Write-SvWarn "This cannot be undone (except from $($script:SV.BackupName))."

        if (-not (Confirm-SvPhrase -Phrase $entry.name -Prompt 'type the name to confirm')) {
            Write-SvDim 'cancelled'
            return
        }

        Remove-SvEntry -Body $context.Body -Id $entry.id
        Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body
        Write-SvOk "deleted $($entry.name)"
    }
    finally { Clear-SvBytes $context.MasterKey }
}

# Backup codes are single-use, so revealing one burns it. The spent index is
# metadata, so marking it never re-encrypts the payload.
function Invoke-SvUseCode {
    param([string]$Name)

    if (-not $Name) { throw 'Usage:  secure-vault --use NAME' }

    $context = Open-SvContext
    try {
        $dataKey = Assert-SvUnlocked -Context $context
        try {
            $entry = Find-SvEntryInContext -Context $context -Name $Name
            if ($entry.type -ne 'codes') { throw "'$($entry.name)' is a '$($entry.type)' entry, not backup codes." }

            $codes = Split-SvCodes (Unprotect-SvValue -DataKey $dataKey -Entry $entry)
            $next = Get-SvNextUnusedCode -Entry $entry -Codes $codes
            if ($null -eq $next) { throw "All $($codes.Count) codes in '$($entry.name)' are used." }

            Set-SvCodeUsed -Entry $entry -Index $next.Index
            Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body

            Write-SvBanner -State 'UNLOCKED' -Detail "$($entry.name) · code $($next.Index + 1) of $($codes.Count)"
            Write-Host "  $($next.Code)"
            Write-Host ''
            Write-SvDim "marked used · $($codes.Count - @($entry.used).Count) left"
        }
        finally { Clear-SvBytes $dataKey }
    }
    finally { Clear-SvBytes $context.MasterKey }
}
