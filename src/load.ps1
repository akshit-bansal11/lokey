# load.ps1 — the load order, in one place, so the CLI and the tests share it.
# Dot-source this file and every secure-vault function lands in your scope.

Set-StrictMode -Version Latest

$svRoot = $PSScriptRoot

foreach ($module in @(
        'config', 'crypto', 'render', 'secure-prompt',
        'vault-file', 'vault-crypto', 'entries',
        'session', 'lockout', 'select'
    )) {
    . (Join-Path $svRoot "$module.ps1")
}

foreach ($module in @('session-commands', 'entry-commands', 'project-commands', 'vault-commands')) {
    . (Join-Path $svRoot "commands\$module.ps1")
}
