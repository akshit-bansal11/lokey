#Requires -Version 7.0
# secure-vault.ps1 — the CLI. Parses one flag, dispatches, prints errors nicely.
# Everything else lives in src/.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'src\load.ps1')

$command = if ($args.Count -gt 0) { ([string]$args[0]).TrimStart('-').ToLowerInvariant() } else { '' }
$argument = if ($args.Count -gt 1 -and -not ([string]$args[1]).StartsWith('-')) { [string]$args[1] } else { '' }
$force = @($args | ForEach-Object { [string]$_ }) -contains '--force'

$known = @(
    '', 'unlock', 'lock', 'status', 'show', 'all', 'use',
    'add', 'import', 'move', 'rm', 'remove', 'delete',
    'project', 'projects', 'rm-project',
    'init', 'rotate', 'emergency', 'help', 'h', '?'
)
if ($command -notin $known) {
    Write-SvError "Unknown command '--$command'."
    Show-SvHelp
    exit 2
}

# Anything that reaches the vault file needs one to exist first.
$needsVault = $command -notin @('init', 'help', 'h', '?', 'status')
if ($needsVault -and -not (Test-SvVaultExists)) {
    Write-SvBanner -State 'NO VAULT' -Detail (Get-SvPath vault)
    Write-SvDim 'create one with:  secure-vault --init'
    exit 1
}

try {
    switch ($command) {
        '' { Invoke-SvOpen }
        'unlock' { Invoke-SvUnlock }
        'lock' { Invoke-SvLock }
        'status' { Invoke-SvStatus }
        'show' { Invoke-SvShow -Name $argument }
        'all' { Invoke-SvShowAll }
        'use' { Invoke-SvUseCode -Name $argument }
        'add' { Invoke-SvAdd }
        'import' { Invoke-SvImport }
        'move' { Invoke-SvMove -Name $argument }
        { $_ -in 'rm', 'remove', 'delete' } { Invoke-SvRemove -Name $argument }
        'projects' { Invoke-SvProjects }
        'project' { Invoke-SvProject -Name $argument }
        'rm-project' { Invoke-SvRemoveProject -Name $argument }
        'init' { Invoke-SvInit }
        'rotate' { Invoke-SvRotate }
        'emergency' { Invoke-SvEmergency -Force:$force }
        { $_ -in 'help', 'h', '?' } { Show-SvHelp }
        # Fires only if $known and this switch ever drift apart.
        default { throw "'--$command' is listed as known but has no handler." }
    }
}
catch {
    Write-Host ''
    Write-SvError $_.Exception.Message
    Write-Host ''
    exit 1
}
