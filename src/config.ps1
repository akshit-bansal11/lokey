# config.ps1 — every tunable constant for secure-vault. No logic lives here.

Set-StrictMode -Version Latest

$script:SV = @{
    Name             = 'secure-vault'
    Format           = 1

    VaultDir         = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.secure-vault'
    StateDir         = Join-Path $env:LOCALAPPDATA 'secure-vault'
    VaultName        = 'vault.sv'
    BackupName       = 'vault.sv.bak'
    SessionName      = 'session.bin'
    LockoutName      = 'lockout.json'

    Kdf              = 'pbkdf2-sha256'
    Iterations       = 600000      # OWASP 2023 floor for PBKDF2-SHA256
    MinIterations    = 100000
    KeyBytes         = 32          # AES-256
    SaltBytes        = 16
    NonceBytes       = 12          # GCM standard
    TagBytes         = 16

    MinPasswordLen   = 10
    OpenTtlMinutes   = 15          # master key: opens metadata only, lower value to an attacker
    UnlockTtlMinutes = 2           # data key: the one that reveals secrets — keep this window small

    MaxFails         = 10
    LockoutMinutes   = 15
    BackoffCapSec    = 30
    WipePasses       = 3

    MaskChar         = [char]0x2022
    MaskMaxDots      = 12

    DefaultProject   = 'default'
    PairSeparator    = ';'
    PairDelimiter    = ','

    Types            = @('kv', 'codes', 'pw')
    TypeHelp         = @{
        kv    = 'key=value environment variable'
        codes = 'one-time backup / recovery codes'
        pw    = 'a password or passphrase'
    }
}

# Test + power-user overrides. Iterations only affect --init/--rotate; opening an
# existing vault always uses the iteration count recorded in its own header.
if ($env:SECUREVAULT_DIR) {
    $script:SV.VaultDir = $env:SECUREVAULT_DIR
    $script:SV.StateDir = $env:SECUREVAULT_DIR
}
if ($env:SECUREVAULT_ITER) {
    $requested = 0
    if ([int]::TryParse($env:SECUREVAULT_ITER, [ref]$requested)) {
        $floor = if ($env:SECUREVAULT_TEST -eq '1') { 1 } else { $script:SV.MinIterations }
        $script:SV.Iterations = [Math]::Max($floor, $requested)
    }
}
