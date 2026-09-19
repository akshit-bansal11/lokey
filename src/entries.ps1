# entries.ps1 — the entry list inside the envelope. Pure data shaping, no I/O.
#
# One shape covers all three types; only the payload and the rendering differ:
#   kv    -> payload is the variable's value
#   pw    -> payload is the password
#   codes -> payload is the codes, newline-separated; spent indexes live in
#            .used (metadata) so burning a code never re-encrypts the payload.
#
# Every entry belongs to exactly one project. Names are unique within a project,
# so DATABASE_URL can exist in both `web` and `api` without a collision.

Set-StrictMode -Version Latest

# .check is sealed by vault-crypto's Set-SvBodyCheck; this file stays free of
# any crypto so it can be tested on plain data.
function New-SvBody {
    @{
        check    = $null
        entries  = @()
        projects = @($script:SV.DefaultProject)
        current  = $script:SV.DefaultProject
    }
}

# Runs on every envelope open, so a vault written before projects existed gains
# them without a format bump or a migration step.
function Initialize-SvBody {
    param([Parameter(Mandatory)][hashtable]$Body)

    # Dot access on a missing hashtable key throws under Set-StrictMode -Latest,
    # and @($null) is a one-element array rather than an empty one. Every field a
    # pre-projects vault can lack is therefore read through the indexer and
    # filtered, not dotted.
    if (-not $Body.ContainsKey('check')) { $Body['check'] = $null }

    $projects = @(@($Body['projects']) | Where-Object { $_ })
    if ($projects.Count -eq 0) { $projects = @($script:SV.DefaultProject) }
    $Body['projects'] = $projects

    if (-not $Body['current'] -or $projects -notcontains $Body['current']) {
        $Body['current'] = $projects[0]
    }

    $Body['entries'] = @(@($Body['entries']) | Where-Object { $_ })
    foreach ($entry in $Body['entries']) {
        if (-not $entry['project'] -or $projects -notcontains $entry['project']) {
            $entry['project'] = $Body['current']
        }
    }
    $Body
}

function New-SvEntry {
    param(
        [Parameter(Mandatory)][ValidateSet('kv', 'codes', 'pw')][string]$Type,
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Project,
        [string]$Note = ''
    )

    $now = (Get-Date).ToString('o')
    @{
        id      = [guid]::NewGuid().ToString('N')
        type    = $Type
        name    = $Name
        project = $Project
        note    = $Note
        used    = @()
        total   = $null   # codes only: how many, so the list can render without decrypting
        created = $now
        updated = $now
        val     = $null
    }
}

function Get-SvEntries {
    param([Parameter(Mandatory)][hashtable]$Body, [string]$Project)

    $pool = @($Body.entries)
    if ($Project) { $pool = @($pool | Where-Object { $_.project -eq $Project }) }
    @($pool | Sort-Object -Property @{ Expression = 'type' }, @{ Expression = 'name' })
}

# Exact name wins; otherwise a unique case-insensitive prefix. Ambiguity is an
# error rather than a guess — picking the wrong secret silently is worse. Looks
# in the active project first, then falls back to the whole vault.
function Find-SvEntry {
    param(
        [Parameter(Mandatory)][hashtable]$Body,
        [Parameter(Mandatory)][string]$Name,
        [string]$Project
    )

    $pool = Get-SvEntries -Body $Body -Project $Project

    $exact = @($pool | Where-Object { $_.name -ieq $Name })
    if ($exact.Count -eq 1) { return $exact[0] }
    if ($exact.Count -gt 1) {
        throw "'$Name' exists in $($exact.Count) projects: $((($exact.project) | Sort-Object) -join ', '). Switch with --project NAME."
    }

    $partial = @($pool | Where-Object { $_.name -ilike "$Name*" })
    if ($partial.Count -eq 1) { return $partial[0] }
    if ($partial.Count -gt 1) {
        throw "'$Name' matches $($partial.Count) entries: $(($partial.name) -join ', ')"
    }

    if ($Project) { return Find-SvEntry -Body $Body -Name $Name }
    throw "No entry named '$Name'."
}

function Add-SvEntry {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][hashtable]$Entry)

    if (@($Body.entries) | Where-Object { $_.name -ieq $Entry.name -and $_.project -eq $Entry.project }) {
        throw "An entry named '$($Entry.name)' already exists in project '$($Entry.project)'."
    }
    $Body.entries = @($Body.entries) + $Entry
}

# A value's associated data is its id and type, never its project, so moving an
# entry between projects needs no re-encryption — and therefore no decrypt
# password. Returns the canonical target project name.
function Move-SvEntry {
    param(
        [Parameter(Mandatory)][hashtable]$Body,
        [Parameter(Mandatory)][hashtable]$Entry,
        [Parameter(Mandatory)][string]$ToProject
    )

    $match = @($Body.projects | Where-Object { $_ -ieq $ToProject })
    if ($match.Count -ne 1) { throw "No project named '$ToProject'. See:  secure-vault --projects" }
    $target = $match[0]

    if ($Entry.project -eq $target) { throw "'$($Entry.name)' is already in '$target'." }
    if (@($Body.entries) | Where-Object { $_.name -ieq $Entry.name -and $_.project -eq $target }) {
        throw "'$target' already has an entry named '$($Entry.name)'. Delete that one first."
    }

    $Entry.project = $target
    $Entry.updated = (Get-Date).ToString('o')
    $target
}

function Remove-SvEntry {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][string]$Id)

    $Body.entries = @(@($Body.entries) | Where-Object { $_.id -ne $Id })
}

# --- projects ----------------------------------------------------------------

function Get-SvProjects {
    param([Parameter(Mandatory)][hashtable]$Body)

    # NOTE: the entry-count key must not be called `count` — PowerShell resolves
    # .Count on a Hashtable to its key count, which would shadow the value.
    @($Body.projects | ForEach-Object {
            @{
                name    = $_
                entries = @(Get-SvEntries -Body $Body -Project $_).Count
                current = ($_ -eq $Body.current)
            }
        })
}

function Add-SvProject {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][string]$Name)

    if ([string]::IsNullOrWhiteSpace($Name)) { throw 'A project name is required.' }
    if ($Body.projects -icontains $Name) { throw "Project '$Name' already exists." }
    $Body.projects = @($Body.projects) + $Name.Trim()
}

function Set-SvCurrentProject {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][string]$Name)

    $match = @($Body.projects | Where-Object { $_ -ieq $Name })
    if ($match.Count -ne 1) { throw "No project named '$Name'. See:  secure-vault --projects" }
    $Body.current = $match[0]
    $match[0]
}

function Remove-SvProject {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][string]$Name)

    if (@($Body.projects).Count -le 1) { throw 'Cannot remove the last project.' }

    $match = @($Body.projects | Where-Object { $_ -ieq $Name })
    if ($match.Count -ne 1) { throw "No project named '$Name'." }
    $target = $match[0]

    $Body.entries = @(@($Body.entries) | Where-Object { $_.project -ne $target })
    $Body.projects = @(@($Body.projects) | Where-Object { $_ -ne $target })
    if ($Body.current -eq $target) { $Body.current = $Body.projects[0] }
    $target
}

# --- backup codes ------------------------------------------------------------

function Split-SvCodes {
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Payload)

    @($Payload -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
}

function Join-SvCodes {
    param([Parameter(Mandatory)][string[]]$Codes)

    $Codes -join "`n"
}

function Get-SvNextUnusedCode {
    param([Parameter(Mandatory)][hashtable]$Entry, [Parameter(Mandatory)][string[]]$Codes)

    $used = @($Entry.used)
    for ($i = 0; $i -lt $Codes.Count; $i++) {
        if ($used -notcontains $i) { return @{ Index = $i; Code = $Codes[$i] } }
    }
    $null
}

function Set-SvCodeUsed {
    param([Parameter(Mandatory)][hashtable]$Entry, [Parameter(Mandatory)][int]$Index)

    $Entry.used = @(@($Entry.used) + $Index | Sort-Object -Unique)
    $Entry.updated = (Get-Date).ToString('o')
}

# --- bulk paste --------------------------------------------------------------

# Parses  key1,value1; key2,value2;  in any mix of newlines and spaces, with or
# without a trailing separator and with or without wrapping quotes. Splits on
# the FIRST delimiter only, so a value may contain commas. A value may not
# contain the separator — that is the format's one real limit.
function ConvertFrom-SvPairText {
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Text)

    $pairs = @()
    $bad = @()

    foreach ($chunk in ($Text -split [regex]::Escape($script:SV.PairSeparator))) {
        $line = $chunk.Trim().Trim('"').Trim()
        if (-not $line) { continue }

        $split = $line.IndexOf($script:SV.PairDelimiter)
        if ($split -lt 1) { $bad += $line; continue }

        $name = $line.Substring(0, $split).Trim().Trim('"').Trim()
        $value = $line.Substring($split + 1).Trim().Trim('"')
        if (-not $name) { $bad += $line; continue }

        $pairs += @{ name = $name; value = $value }
    }

    @{ pairs = $pairs; bad = $bad }
}
