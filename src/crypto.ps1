# crypto.ps1 — the only file that touches cryptographic primitives.
# AES-256-GCM + PBKDF2-SHA256, both straight from .NET. No third-party code.

Set-StrictMode -Version Latest

function New-SvRandomBytes {
    param([Parameter(Mandatory)][int]$Count)

    $bytes = [byte[]]::new($Count)
    [System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    , $bytes
}

function Clear-SvBytes {
    param([byte[]]$Bytes)

    if ($null -ne $Bytes) {
        [System.Security.Cryptography.CryptographicOperations]::ZeroMemory($Bytes)
    }
}

function Test-SvBytesEqual {
    param([byte[]]$Left, [byte[]]$Right)

    [System.Security.Cryptography.CryptographicOperations]::FixedTimeEquals($Left, $Right)
}

function Get-SvDerivedKey {
    param(
        [Parameter(Mandatory)][byte[]]$Password,
        [Parameter(Mandatory)][byte[]]$Salt,
        [Parameter(Mandatory)][int]$Iterations
    )

    , [System.Security.Cryptography.Rfc2898DeriveBytes]::Pbkdf2(
        $Password,
        $Salt,
        $Iterations,
        [System.Security.Cryptography.HashAlgorithmName]::SHA256,
        $script:SV.KeyBytes
    )
}

# Returns @{ n = nonce; c = ciphertext; t = tag } — all base64, JSON-safe.
function Protect-SvBytes {
    param(
        [Parameter(Mandatory)][byte[]]$Key,
        # AllowEmptyCollection: a zero-length secret is legitimate (an imported
        # `KEY,;` pair), and Mandatory alone rejects an empty array.
        [Parameter(Mandatory)][AllowEmptyCollection()][byte[]]$Plaintext,
        [byte[]]$Aad = [byte[]]::new(0)
    )

    $nonce = New-SvRandomBytes $script:SV.NonceBytes
    $cipher = [byte[]]::new($Plaintext.Length)
    $tag = [byte[]]::new($script:SV.TagBytes)

    $gcm = [System.Security.Cryptography.AesGcm]::new($Key, $script:SV.TagBytes)
    try { $gcm.Encrypt($nonce, $Plaintext, $cipher, $tag, $Aad) }
    finally { $gcm.Dispose() }

    @{
        n = [Convert]::ToBase64String($nonce)
        c = [Convert]::ToBase64String($cipher)
        t = [Convert]::ToBase64String($tag)
    }
}

# Throws CryptographicException on a wrong key or any tampering — callers treat
# that as "wrong password", which is exactly what GCM authentication buys us.
function Unprotect-SvBytes {
    param(
        [Parameter(Mandatory)][byte[]]$Key,
        [Parameter(Mandatory)]$Sealed,
        [byte[]]$Aad = [byte[]]::new(0)
    )

    $nonce = [Convert]::FromBase64String($Sealed.n)
    $cipher = [Convert]::FromBase64String($Sealed.c)
    $tag = [Convert]::FromBase64String($Sealed.t)
    $plain = [byte[]]::new($cipher.Length)

    $gcm = [System.Security.Cryptography.AesGcm]::new($Key, $script:SV.TagBytes)
    try { $gcm.Decrypt($nonce, $cipher, $tag, $plain, $Aad) }
    finally { $gcm.Dispose() }

    , $plain
}

# Concatenate secrets at the byte level so multi-part payloads (backup codes)
# never have to become a managed string first.
function Join-SvBytes {
    param([Parameter(Mandatory)][System.Collections.Generic.List[byte[]]]$Parts, [byte]$Separator = 0x0A)

    $out = [System.Collections.Generic.List[byte]]::new()
    for ($i = 0; $i -lt $Parts.Count; $i++) {
        if ($i -gt 0) { $out.Add($Separator) }
        $out.AddRange($Parts[$i])
    }
    , $out.ToArray()
}

function Get-SvSealedSize {
    param([Parameter(Mandatory)]$Sealed)

    [Convert]::FromBase64String($Sealed.c).Length
}
