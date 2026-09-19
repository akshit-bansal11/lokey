# project-commands.ps1 — projects group entries so the same name can live in
# more than one place. Switching only needs the master password; the active
# project is metadata, not a secret.

Set-StrictMode -Version Latest

function Invoke-SvProjects {
    $context = Open-SvContext
    try {
        $projects = Get-SvProjects -Body $context.Body
        Write-SvBanner -State 'OPEN' -Detail "$($projects.Count) project(s) · active: $($context.Body.current)"
        Show-SvProjectTable -Projects $projects
    }
    finally { Clear-SvBytes $context.MasterKey }
}

function Invoke-SvProject {
    param([string]$Name)

    if (-not $Name) { Invoke-SvProjects; return }

    $context = Open-SvContext
    try {
        if ($context.Body.projects -inotcontains $Name) {
            Write-SvBanner -State 'OPEN' -Detail "no project named '$Name'"
            if ((Read-SvLine -Prompt "create it? [y/N]") -inotmatch '^y') {
                Write-SvDim 'cancelled'
                return
            }
            Add-SvProject -Body $context.Body -Name $Name
        }

        $switched = Set-SvCurrentProject -Body $context.Body -Name $Name
        Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body

        $count = @(Get-SvEntries -Body $context.Body -Project $switched).Count
        Write-SvOk "active project: $switched ($count entries)"
    }
    finally { Clear-SvBytes $context.MasterKey }
}

function Invoke-SvRemoveProject {
    param([string]$Name)

    if (-not $Name) { throw 'Usage:  secure-vault --rm-project NAME' }

    $context = Open-SvContext
    try {
        $match = @($context.Body.projects | Where-Object { $_ -ieq $Name })
        if ($match.Count -ne 1) { throw "No project named '$Name'. See:  secure-vault --projects" }

        $target = $match[0]
        $count = @(Get-SvEntries -Body $context.Body -Project $target).Count

        Write-SvBanner -State 'OPEN' -Detail "delete project $target"
        Write-SvWarn "This deletes the project and all $count entries in it. Recoverable only from $($script:SV.BackupName)."
        if (-not (Confirm-SvPhrase -Phrase $target -Prompt 'type the project name to confirm')) {
            Write-SvDim 'cancelled'
            return
        }

        $removed = Remove-SvProject -Body $context.Body -Name $target
        Save-SvVault -Vault $context.Vault -MasterKey $context.MasterKey -Body $context.Body
        Write-SvOk "deleted project $removed · active project is now $($context.Body.current)"
    }
    finally { Clear-SvBytes $context.MasterKey }
}
