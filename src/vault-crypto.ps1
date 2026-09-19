# vault-crypto.ps1 — the two-layer envelope.
#
#   master password  -> masterKey -> opens the envelope: names, types, notes.
#   decrypt password -> dataKey   -> opens each value inside that envelope.
#
# Value ciphertexts live *inside* the master envelope, so reading a secret needs
# both passwords, and holding only one of them gets you nothing useful.

Set-StrictMode -Version Latest

# Header fields are authenticated as GCM associated data, so nobody can swap in
# weaker salts or a lower iteration count without breaking the tag.
function Get-SvHeaderAad {
    param([Parameter(Mandatory)]$Vault)

    $text = 'sv{0}|{1}|{2}|{3}|{4}|{5}|{6}' -f `
        $Vault.v, $Vault.kdf.alg, $Vault.kdf.iter, $Vault.kdf.sm, $Vault.kdf.sd, $Vault.kdf.se, $Vault.erase
    , [System.Text.Encoding]::UTF8.GetBytes($text)
}

function Get-SvValueAad {
    param([Parameter(Mandatory)][string]$Id, [Parameter(Mandatory)][string]$Type)

    , [System.Text.Encoding]::UTF8.GetBytes("sv-val|$Id|$Type")
}

function Get-SvMasterKey {
    param([Parameter(Mandatory)][byte[]]$Password, [Parameter(Mandatory)]$Vault)

    , (Get-SvDerivedKey -Password $Password -Salt ([Convert]::FromBase64String($Vault.kdf.sm)) -Iterations $Vault.kdf.iter)
}

function Get-SvDataKey {
    param([Parameter(Mandatory)][byte[]]$Password, [Parameter(Mandatory)]$Vault)

    , (Get-SvDerivedKey -Password $Password -Salt ([Convert]::FromBase64String($Vault.kdf.sd)) -Iterations $Vault.kdf.iter)
}

function Get-SvEraseVerifier {
    param([Parameter(Mandatory)][byte[]]$Password, [Parameter(Mandatory)]$Vault)

    , (Get-SvDerivedKey -Password $Password -Salt ([Convert]::FromBase64String($Vault.kdf.se)) -Iterations $Vault.kdf.iter)
}

# The deletion verifier sits in the cleartext header on purpose: --emergency must
# work with the deletion password alone. Cracking it only buys an attacker the
# ability to destroy a file they already have write access to.
function Test-SvErasePassword {
    param([Parameter(Mandatory)][byte[]]$Password, [Parameter(Mandatory)]$Vault)

    $candidate = Get-SvEraseVerifier -Password $Password -Vault $Vault
    try { Test-SvBytesEqual $candidate ([Convert]::FromBase64String($Vault.erase)) }
    finally { Clear-SvBytes $candidate }
}

function New-SvVaultHeader {
    param([Parameter(Mandatory)][byte[]]$ErasePassword, [int]$Iterations = $script:SV.Iterations)

    $saltMaster = New-SvRandomBytes $script:SV.SaltBytes
    $saltData = New-SvRandomBytes $script:SV.SaltBytes
    $saltErase = New-SvRandomBytes $script:SV.SaltBytes

    $header = @{
        v     = $script:SV.Format
        kdf   = @{
            alg  = $script:SV.Kdf
            iter = $Iterations
            sm   = [Convert]::ToBase64String($saltMaster)
            sd   = [Convert]::ToBase64String($saltData)
            se   = [Convert]::ToBase64String($saltErase)
        }
        meta  = @{
            created = (Get-Date).ToString('o')
            updated = (Get-Date).ToString('o')
            rotated = (Get-Date).ToString('o')
        }
        erase = ''
        env   = $null
    }

    $verifier = Get-SvEraseVerifier -Password $ErasePassword -Vault $header
    try { $header.erase = [Convert]::ToBase64String($verifier) }
    finally { Clear-SvBytes $verifier }
    $header
}

function Open-SvEnvelope {
    param([Parameter(Mandatory)]$Vault, [Parameter(Mandatory)][byte[]]$MasterKey)

    $plain = Unprotect-SvBytes -Key $MasterKey -Sealed $Vault.env -Aad (Get-SvHeaderAad $Vault)
    try {
        # Initialize-SvBody backfills anything a vault written by an older build
        # is missing, so opening is the single migration point.
        Initialize-SvBody -Body ([System.Text.Encoding]::UTF8.GetString($plain) | ConvertFrom-Json -AsHashtable)
    }
    finally { Clear-SvBytes $plain }
}

function Close-SvEnvelope {
    param(
        [Parameter(Mandatory)]$Vault,
        [Parameter(Mandatory)][byte[]]$MasterKey,
        [Parameter(Mandatory)][hashtable]$Body
    )

    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($Body | ConvertTo-Json -Depth 12 -Compress))
    try { Protect-SvBytes -Key $MasterKey -Plaintext $bytes -Aad (Get-SvHeaderAad $Vault) }
    finally { Clear-SvBytes $bytes }
}

# An empty vault has no value to test a decrypt password against, so the body
# carries one sealed known plaintext. This is not a new leak: every value's GCM
# tag is already a verification oracle, and reaching `check` needs the master
# password first, since it lives inside the master envelope.
function Get-SvCheckAad {
    , [System.Text.Encoding]::UTF8.GetBytes('sv-check')
}

function Set-SvBodyCheck {
    param([Parameter(Mandatory)][hashtable]$Body, [Parameter(Mandatory)][byte[]]$DataKey)

    $probe = [System.Text.Encoding]::UTF8.GetBytes('secure-vault/v1')
    $Body.check = Protect-SvBytes -Key $DataKey -Plaintext $probe -Aad (Get-SvCheckAad)
}

function Test-SvDataKey {
    param([Parameter(Mandatory)][byte[]]$DataKey, [Parameter(Mandatory)][hashtable]$Body)

    if ($null -eq $Body.check) { return $false }
    try {
        $plain = Unprotect-SvBytes -Key $DataKey -Sealed $Body.check -Aad (Get-SvCheckAad)
        try { Test-SvBytesEqual $plain ([System.Text.Encoding]::UTF8.GetBytes('secure-vault/v1')) }
        finally { Clear-SvBytes $plain }
    }
    catch [System.Security.Cryptography.CryptographicException] { $false }
}

function Get-SvDecryptedValues {
    param(
        [Parameter(Mandatory)][hashtable]$Body,
        [Parameter(Mandatory)][byte[]]$DataKey,
        [Parameter(Mandatory)][array]$Entries
    )

    $values = @{}
    foreach ($entry in $Entries) { $values[$entry.id] = Unprotect-SvValue -DataKey $DataKey -Entry $entry }
    $values
}

function Protect-SvValueBytes {
    param(
        [Parameter(Mandatory)][byte[]]$DataKey,
        [Parameter(Mandatory)][hashtable]$Entry,
        [Parameter(Mandatory)][AllowEmptyCollection()][byte[]]$Plaintext
    )

    Protect-SvBytes -Key $DataKey -Plaintext $Plaintext -Aad (Get-SvValueAad $Entry.id $Entry.type)
}

function Protect-SvValue {
    param(
        [Parameter(Mandatory)][byte[]]$DataKey,
        [Parameter(Mandatory)][hashtable]$Entry,
        [Parameter(Mandatory)][AllowEmptyString()][string]$Plaintext
    )

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Plaintext)
    try { Protect-SvValueBytes -DataKey $DataKey -Entry $Entry -Plaintext $bytes }
    finally { Clear-SvBytes $bytes }
}

function Unprotect-SvValue {
    param([Parameter(Mandatory)][byte[]]$DataKey, [Parameter(Mandatory)][hashtable]$Entry)

    $plain = Unprotect-SvBytes -Key $DataKey -Sealed $Entry.val -Aad (Get-SvValueAad $Entry.id $Entry.type)
    try { [System.Text.Encoding]::UTF8.GetString($plain) }
    finally { Clear-SvBytes $plain }
}

function Save-SvVault {
    param(
        [Parameter(Mandatory)][hashtable]$Vault,
        [Parameter(Mandatory)][byte[]]$MasterKey,
        [Parameter(Mandatory)][hashtable]$Body
    )

    $Vault.meta.updated = (Get-Date).ToString('o')
    $Vault.env = Close-SvEnvelope -Vault $Vault -MasterKey $MasterKey -Body $Body
    Write-SvVaultFile -Vault $Vault
}
