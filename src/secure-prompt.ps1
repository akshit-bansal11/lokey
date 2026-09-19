# secure-prompt.ps1 — turning keystrokes into key material without ever building
# a managed System.String that the GC could leave lying around in the heap.

Set-StrictMode -Version Latest

function ConvertTo-SvPasswordBytes {
    param([Parameter(Mandatory)][System.Security.SecureString]$Secure)

    $bstr = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($Secure)
    try {
        $utf16 = [byte[]]::new($Secure.Length * 2)
        [System.Runtime.InteropServices.Marshal]::Copy($bstr, $utf16, 0, $utf16.Length)
        try {
            , [System.Text.Encoding]::Convert(
                [System.Text.Encoding]::Unicode,
                [System.Text.Encoding]::UTF8,
                $utf16)
        }
        finally { Clear-SvBytes $utf16 }
    }
    finally { [System.Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr) }
}

function Read-SvPassword {
    param(
        [Parameter(Mandatory)][string]$Prompt,
        [switch]$Confirm,
        [switch]$EnforceLength,
        [switch]$AllowEmpty
    )

    if ([Console]::IsInputRedirected) {
        throw 'secure-vault needs an interactive console to read a password.'
    }

    while ($true) {
        $first = Read-Host -Prompt "  $Prompt" -AsSecureString
        if ($first.Length -eq 0) {
            $first.Dispose()
            if ($AllowEmpty) { return $null }
            Write-SvWarn 'Empty password. Try again.'
            continue
        }
        if ($EnforceLength -and $first.Length -lt $script:SV.MinPasswordLen) {
            $first.Dispose()
            Write-SvWarn "Use at least $($script:SV.MinPasswordLen) characters."
            continue
        }

        $bytes = ConvertTo-SvPasswordBytes $first
        $first.Dispose()
        if (-not $Confirm) { return , $bytes }

        $second = Read-Host -Prompt "  $Prompt (again)" -AsSecureString
        $check = ConvertTo-SvPasswordBytes $second
        $second.Dispose()

        $matched = Test-SvBytesEqual $bytes $check
        Clear-SvBytes $check
        if ($matched) { return , $bytes }

        Clear-SvBytes $bytes
        Write-SvWarn 'They did not match. Try again.'
    }
}

# Collects secret lines until a blank one. A multi-line paste lands here
# naturally: each pasted line satisfies one prompt, then Enter on the empty
# prompt ends it.
function Read-SvSecretLines {
    param([Parameter(Mandatory)][string]$Prompt)

    $parts = [System.Collections.Generic.List[byte[]]]::new()
    while ($true) {
        $line = Read-SvPassword -Prompt "$Prompt $($parts.Count + 1)" -AllowEmpty
        if ($null -eq $line) { break }
        $parts.Add($line)
    }
    , $parts
}

function Read-SvLine {
    param([Parameter(Mandatory)][string]$Prompt, [string]$Default = '')

    $answer = Read-Host -Prompt "  $Prompt"
    if ([string]::IsNullOrWhiteSpace($answer)) { return $Default }
    $answer.Trim()
}

function Confirm-SvPhrase {
    param([Parameter(Mandatory)][string]$Phrase, [Parameter(Mandatory)][string]$Prompt)

    (Read-SvLine -Prompt $Prompt) -ceq $Phrase
}
