# secure-vault.tests.ps1 — one runnable check for every non-trivial path.
# No framework: run it with  pwsh -File tests\secure-vault.tests.ps1
#
# Everything here works at the byte level, never through the interactive
# prompts, which is exactly why the prompt layer is a separate file.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$env:SECUREVAULT_TEST = '1'
$env:SECUREVAULT_ITER = '1'   # PBKDF2 is the slow part; correctness does not need 600k
$env:SECUREVAULT_DIR = Join-Path ([System.IO.Path]::GetTempPath()) "sv-test-$([guid]::NewGuid().ToString('N'))"

. (Join-Path $PSScriptRoot '..\src\load.ps1')

$script:passed = 0
$script:failed = 0

function Assert-True {
    param([bool]$Condition, [Parameter(Mandatory)][string]$Label)

    if ($Condition) { $script:passed++; Write-Host "  ok    $Label" -ForegroundColor DarkGray }
    else { $script:failed++; Write-Host "  FAIL  $Label" -ForegroundColor Red }
}

function Assert-Throws {
    param([Parameter(Mandatory)][scriptblock]$Action, [Parameter(Mandatory)][string]$Label)

    try { $null = & $Action; Assert-True $false "$Label (expected a throw)" }
    catch { Assert-True $true $Label }
}

function Get-Utf8 { param([string]$Text) , [System.Text.Encoding]::UTF8.GetBytes($Text) }

function Copy-Sealed { param($Sealed) @{ n = $Sealed.n; c = $Sealed.c; t = $Sealed.t } }

Write-Host "`n  secure-vault self-check  ($env:SECUREVAULT_DIR)`n"

# --- crypto primitives -------------------------------------------------------
Write-Host '  crypto' -ForegroundColor Cyan
$key = New-SvRandomBytes 32
$plain = Get-Utf8 'postgres://user:pw@localhost/app'
$aad = Get-Utf8 'bound-context'

$sealed = Protect-SvBytes -Key $key -Plaintext $plain -Aad $aad
Assert-True ((((Unprotect-SvBytes -Key $key -Sealed $sealed -Aad $aad) -join ',') -eq ($plain -join ','))) 'gcm round-trip returns the same bytes'
Assert-True ($sealed.c -ne [Convert]::ToBase64String($plain)) 'ciphertext is not the plaintext'
Assert-True ((Protect-SvBytes -Key $key -Plaintext $plain -Aad $aad).n -ne $sealed.n) 'a fresh nonce per seal'

Assert-Throws { Unprotect-SvBytes -Key (New-SvRandomBytes 32) -Sealed $sealed -Aad $aad } 'wrong key is rejected'
Assert-Throws { Unprotect-SvBytes -Key $key -Sealed $sealed -Aad (Get-Utf8 'other-context') } 'wrong aad is rejected'

$tampered = Copy-Sealed $sealed
$bytes = [Convert]::FromBase64String($tampered.c)
$bytes[0] = $bytes[0] -bxor 0xFF
$tampered.c = [Convert]::ToBase64String($bytes)
Assert-Throws { Unprotect-SvBytes -Key $key -Sealed $tampered -Aad $aad } 'flipped ciphertext bit is detected'

Assert-True (Test-SvBytesEqual (Get-Utf8 'same') (Get-Utf8 'same')) 'fixed-time compare matches equal bytes'
Assert-True (-not (Test-SvBytesEqual (Get-Utf8 'same') (Get-Utf8 'diff'))) 'fixed-time compare rejects different bytes'

$derivedA = Get-SvDerivedKey -Password (Get-Utf8 'pw') -Salt (Get-Utf8 'salt-one') -Iterations 10
$derivedB = Get-SvDerivedKey -Password (Get-Utf8 'pw') -Salt (Get-Utf8 'salt-two') -Iterations 10
Assert-True (-not (Test-SvBytesEqual $derivedA $derivedB)) 'different salts derive different keys'
Assert-True ($derivedA.Length -eq 32) 'derived key is 256 bits'

$zeroed = Get-Utf8 'secret'
Clear-SvBytes $zeroed
Assert-True ((@($zeroed | Where-Object { $_ -ne 0 }).Count) -eq 0) 'Clear-SvBytes zeroes the buffer'

$joined = Join-SvBytes -Parts ([System.Collections.Generic.List[byte[]]]@((Get-Utf8 'aaa'), (Get-Utf8 'bbb')))
Assert-True ([System.Text.Encoding]::UTF8.GetString($joined) -eq "aaa`nbbb") 'Join-SvBytes separates with newline'

# --- entries -----------------------------------------------------------------
Write-Host '  entries' -ForegroundColor Cyan
$body = New-SvBody
$kv = New-SvEntry -Type kv -Name 'DATABASE_URL' -Project 'default' -Note 'prod'
$codes = New-SvEntry -Type codes -Name 'GITHUB_2FA' -Project 'default'
$pw = New-SvEntry -Type pw -Name 'DATABASE_ADMIN' -Project 'default'
Add-SvEntry -Body $body -Entry $kv
Add-SvEntry -Body $body -Entry $codes
Add-SvEntry -Body $body -Entry $pw

Assert-True (@($body.entries).Count -eq 3) 'three entries added'
Assert-Throws { Add-SvEntry -Body $body -Entry (New-SvEntry -Type kv -Name 'database_url' -Project 'default') } 'duplicate name is rejected (case-insensitive)'
Assert-True ((Find-SvEntry -Body $body -Name 'DATABASE_URL').id -eq $kv.id) 'exact name wins'
Assert-True ((Find-SvEntry -Body $body -Name 'github').id -eq $codes.id) 'unique prefix resolves'
Assert-Throws { Find-SvEntry -Body $body -Name 'DATABASE' } 'ambiguous prefix throws instead of guessing'
Assert-Throws { Find-SvEntry -Body $body -Name 'NOPE' } 'missing name throws'

$codeList = @('aaa-111', 'bbb-222', 'ccc-333')
Assert-True ((Split-SvCodes (Join-SvCodes $codeList)).Count -eq 3) 'codes survive join/split'
Assert-True ((Get-SvNextUnusedCode -Entry $codes -Codes $codeList).Index -eq 0) 'first unused code is index 0'
Set-SvCodeUsed -Entry $codes -Index 0
Assert-True ((Get-SvNextUnusedCode -Entry $codes -Codes $codeList).Code -eq 'bbb-222') 'burning a code advances to the next'
Set-SvCodeUsed -Entry $codes -Index 0
Assert-True (@($codes.used).Count -eq 1) 'burning the same code twice does not duplicate'

Remove-SvEntry -Body $body -Id $pw.id
Assert-True (@($body.entries).Count -eq 2) 'remove drops exactly one entry'

# --- projects ----------------------------------------------------------------
Write-Host '  projects' -ForegroundColor Cyan
Add-SvProject -Body $body -Name 'web'
Assert-True (@($body.projects) -join ',' -eq 'default,web') 'project added'
Assert-Throws { Add-SvProject -Body $body -Name 'WEB' } 'duplicate project is rejected'
Assert-True ((Set-SvCurrentProject -Body $body -Name 'web') -eq 'web') 'switching returns the canonical name'
Assert-Throws { Set-SvCurrentProject -Body $body -Name 'nope' } 'switching to an unknown project throws'

# the whole point of projects: the same name in two of them
$webUrl = New-SvEntry -Type kv -Name 'DATABASE_URL' -Project 'web'
Add-SvEntry -Body $body -Entry $webUrl
Assert-True (@($body.entries).Count -eq 3) 'same name is allowed in a different project'
Assert-True ((Find-SvEntry -Body $body -Name 'DATABASE_URL' -Project 'web').id -eq $webUrl.id) 'scoped lookup finds the project copy'
Assert-True ((Find-SvEntry -Body $body -Name 'DATABASE_URL' -Project 'default').id -eq $kv.id) 'scoped lookup finds the default copy'
Assert-Throws { Find-SvEntry -Body $body -Name 'DATABASE_URL' } 'an unscoped lookup across two projects refuses to guess'
Assert-True ((Find-SvEntry -Body $body -Name 'GITHUB_2FA' -Project 'web').id -eq $codes.id) 'lookup falls back to other projects when scoped search misses'
Assert-True (@(Get-SvEntries -Body $body -Project 'web').Count -eq 1) 'entries filter by project'

$counts = Get-SvProjects -Body $body
Assert-True ((@($counts | Where-Object { $_.name -eq 'default' })[0].entries) -eq 2) 'project listing counts entries'
Assert-True ((@($counts | Where-Object { $_.current })[0].name) -eq 'web') 'project listing marks the active one'

Assert-True ((Move-SvEntry -Body $body -Entry $codes -ToProject 'web') -eq 'web') 'entry moves to another project'
Assert-True ($codes.project -eq 'web') 'moved entry records its new project'
Assert-True (@(Get-SvEntries -Body $body -Project 'web').Count -eq 2) 'target project gained the entry'
Assert-Throws { Move-SvEntry -Body $body -Entry $codes -ToProject 'web' } 'moving into the project it is already in throws'
Assert-Throws { Move-SvEntry -Body $body -Entry $codes -ToProject 'nope' } 'moving to an unknown project throws'
Assert-Throws { Move-SvEntry -Body $body -Entry $webUrl -ToProject 'default' } 'a move that would duplicate a name is refused'
$null = Move-SvEntry -Body $body -Entry $codes -ToProject 'default'
Assert-True (@(Get-SvEntries -Body $body -Project 'default').Count -eq 2) 'entry moves back'

Assert-True ((Remove-SvProject -Body $body -Name 'web') -eq 'web') 'project removed'
Assert-True (@($body.entries).Count -eq 2) 'removing a project removes its entries'
Assert-True ($body.current -eq 'default') 'removing the active project falls back'
Assert-Throws { Remove-SvProject -Body $body -Name 'default' } 'the last project cannot be removed'

# a vault written before projects existed must open without a migration step
$legacy = Initialize-SvBody -Body @{
    check   = $null
    entries = @(@{ id = 'x'; type = 'kv'; name = 'OLD_KEY'; note = ''; used = @(); val = $null })
}
Assert-True ((@($legacy.projects) -join ',') -eq 'default') 'legacy body gains a default project'
Assert-True ($legacy.current -eq 'default') 'legacy body gains an active project'
Assert-True (@($legacy.entries)[0].project -eq 'default') 'legacy entries are backfilled'

# --- bulk paste parsing ------------------------------------------------------
Write-Host '  import' -ForegroundColor Cyan
$parsed = ConvertFrom-SvPairText -Text "`"`nkey1,value1;`nkey2,value2;`nkey3,value3;`n`""
Assert-True (@($parsed.pairs).Count -eq 3) 'three pairs parsed from the quoted block'
Assert-True (@($parsed.bad).Count -eq 0) 'no bad lines in a clean block'
Assert-True (@($parsed.pairs)[1].name -eq 'key2' -and @($parsed.pairs)[1].value -eq 'value2') 'name and value split correctly'

$parsed = ConvertFrom-SvPairText -Text 'A,1; B,2'
Assert-True (@($parsed.pairs).Count -eq 2) 'a missing trailing separator is fine'

$parsed = ConvertFrom-SvPairText -Text 'DATABASE_URL,postgres://u:p@h/db?opts=a,b,c;'
Assert-True (@($parsed.pairs)[0].value -eq 'postgres://u:p@h/db?opts=a,b,c') 'only the first comma splits, so values may contain commas'

$parsed = ConvertFrom-SvPairText -Text '  KEY  ,  spaced value  ;'
Assert-True (@($parsed.pairs)[0].name -eq 'KEY' -and @($parsed.pairs)[0].value -eq 'spaced value') 'surrounding whitespace is trimmed'

$parsed = ConvertFrom-SvPairText -Text 'GOOD,1; no-comma-here; ,novalue;'
Assert-True (@($parsed.pairs).Count -eq 1) 'malformed lines are not silently imported'
Assert-True (@($parsed.bad).Count -eq 2) 'malformed lines are reported'

Assert-True (@((ConvertFrom-SvPairText -Text '').pairs).Count -eq 0) 'empty text parses to nothing'

# --- masking -----------------------------------------------------------------
# The list view must render without any decryption at all, so masks are built
# from the ciphertext. These run with a key that is never used to decrypt.
Write-Host '  masking' -ForegroundColor Cyan
$maskKey = New-SvRandomBytes 32

$longEntry = New-SvEntry -Type kv -Name 'LONG' -Project 'default'
$longEntry.val = Protect-SvValue -DataKey $maskKey -Entry $longEntry -Plaintext 'postgres://user:pw@localhost/app'
$masked = Get-SvMaskedEntry -Entry $longEntry
Assert-True ($masked -notmatch '[A-Za-z0-9:/@.]') 'mask leaks no character of the value'
Assert-True ($masked.Length -eq $script:SV.MaskMaxDots) 'long values mask to the cap'

$shortEntry = New-SvEntry -Type kv -Name 'SHORT' -Project 'default'
$shortEntry.val = Protect-SvValue -DataKey $maskKey -Entry $shortEntry -Plaintext 'abc'
Assert-True ((Get-SvMaskedEntry -Entry $shortEntry).Length -eq 3) 'short values mask to their own length'

$emptyEntry = New-SvEntry -Type kv -Name 'EMPTY' -Project 'default'
$emptyEntry.val = Protect-SvValue -DataKey $maskKey -Entry $emptyEntry -Plaintext ''
Assert-True ((Get-SvMaskedEntry -Entry $emptyEntry) -eq '(empty)') 'empty value is labelled'
Assert-True ((Unprotect-SvValue -DataKey $maskKey -Entry $emptyEntry) -eq '') 'a zero-length secret round-trips'

$maskCodes = New-SvEntry -Type codes -Name 'MASK_CODES' -Project 'default'
$maskCodes.total = 3
Set-SvCodeUsed -Entry $maskCodes -Index 0
Assert-True ((Get-SvMaskedEntry -Entry $maskCodes) -eq '3 codes, 2 unused') 'codes mask reports usage without decrypting'
Assert-True ($null -eq $maskCodes.val) 'a codes mask needs no ciphertext at all'

# --- backoff -----------------------------------------------------------------
Write-Host '  backoff' -ForegroundColor Cyan
Assert-True ((Get-SvBackoffSeconds 0) -eq 0) 'no wait before the first failure'
Assert-True ((Get-SvBackoffSeconds 1) -eq 1) 'first failure waits 1s'
Assert-True ((Get-SvBackoffSeconds 3) -eq 4) 'third failure waits 4s'
Assert-True ((Get-SvBackoffSeconds 20) -eq $script:SV.BackoffCapSec) 'backoff is capped'

# --- full vault lifecycle ----------------------------------------------------
Write-Host '  vault' -ForegroundColor Cyan
$masterPw = Get-Utf8 'master-password-1'
$dataPw = Get-Utf8 'decrypt-password-2'
$erasePw = Get-Utf8 'deletion-password-3'

$vault = New-SvVaultHeader -ErasePassword $erasePw
$masterKey = Get-SvMasterKey -Password $masterPw -Vault $vault
$dataKey = Get-SvDataKey -Password $dataPw -Vault $vault

$live = New-SvBody
Set-SvBodyCheck -Body $live -DataKey $dataKey
$urlEntry = New-SvEntry -Type kv -Name 'DATABASE_URL' -Project 'default' -Note 'prod'
Add-SvEntry -Body $live -Entry $urlEntry
$urlEntry.val = Protect-SvValue -DataKey $dataKey -Entry $urlEntry -Plaintext 'postgres://user:pw@localhost/app'
$codeEntry = New-SvEntry -Type codes -Name 'GITHUB_2FA' -Project 'default'
Add-SvEntry -Body $live -Entry $codeEntry
$codeEntry.val = Protect-SvValue -DataKey $dataKey -Entry $codeEntry -Plaintext (Join-SvCodes $codeList)

Save-SvVault -Vault $vault -MasterKey $masterKey -Body $live
Assert-True (Test-SvVaultExists) 'vault file written'

$raw = Get-Content -LiteralPath (Get-SvPath vault) -Raw
Assert-True (-not $raw.Contains('DATABASE_URL')) 'entry names are not readable on disk'
Assert-True (-not $raw.Contains('postgres')) 'values are not readable on disk'
Assert-True (-not $raw.Contains('aaa-111')) 'backup codes are not readable on disk'

$reloaded = Read-SvVaultFile
$reloadedMaster = Get-SvMasterKey -Password $masterPw -Vault $reloaded
$reloadedBody = Open-SvEnvelope -Vault $reloaded -MasterKey $reloadedMaster
Assert-True (@($reloadedBody.entries).Count -eq 2) 'master password opens the envelope'

$reloadedData = Get-SvDataKey -Password $dataPw -Vault $reloaded
Assert-True (Test-SvDataKey -DataKey $reloadedData -Body $reloadedBody) 'decrypt password verifies against the check probe'
Assert-True (-not (Test-SvDataKey -DataKey (Get-SvDataKey -Password (Get-Utf8 'wrong') -Vault $reloaded) -Body $reloadedBody)) 'wrong decrypt password is rejected on an empty-ish vault'

$found = Find-SvEntry -Body $reloadedBody -Name 'DATABASE_URL'
Assert-True ((Unprotect-SvValue -DataKey $reloadedData -Entry $found) -eq 'postgres://user:pw@localhost/app') 'value survives a full save/load cycle'
Assert-True ((Split-SvCodes (Unprotect-SvValue -DataKey $reloadedData -Entry (Find-SvEntry -Body $reloadedBody -Name 'GITHUB_2FA'))).Count -eq 3) 'codes survive a full save/load cycle'

Assert-Throws { Open-SvEnvelope -Vault $reloaded -MasterKey (Get-SvMasterKey -Password (Get-Utf8 'wrong') -Vault $reloaded) } 'wrong master password cannot open the envelope'
Assert-Throws { Unprotect-SvValue -DataKey $reloadedMaster -Entry $found } 'the master key cannot decrypt a value'

# header fields are authenticated: editing one invalidates the envelope
$mutated = Read-SvVaultFile
$mutated.kdf.iter = 1000
Assert-Throws { Open-SvEnvelope -Vault $mutated -MasterKey $reloadedMaster } 'downgrading the iteration count breaks the tag'

# value ciphertexts are bound to their entry id, so they cannot be swapped
$swapped = Find-SvEntry -Body $reloadedBody -Name 'GITHUB_2FA'
$stolen = @{ id = $swapped.id; type = $swapped.type; val = (Copy-Sealed $found.val) }
Assert-Throws { Unprotect-SvValue -DataKey $reloadedData -Entry $stolen } 'a value cannot be moved to another entry'

# moving must not require re-encryption, so a moved value still decrypts
Add-SvProject -Body $reloadedBody -Name 'staging'
$null = Move-SvEntry -Body $reloadedBody -Entry $found -ToProject 'staging'
Assert-True ((Unprotect-SvValue -DataKey $reloadedData -Entry $found) -eq 'postgres://user:pw@localhost/app') 'a moved value still decrypts — project is not part of the AAD'
$null = Move-SvEntry -Body $reloadedBody -Entry $found -ToProject 'default'

Assert-True (Test-SvErasePassword -Password $erasePw -Vault $reloaded) 'deletion password verifies'
Assert-True (-not (Test-SvErasePassword -Password $masterPw -Vault $reloaded)) 'master password is not the deletion password'

Save-SvVault -Vault $reloaded -MasterKey $reloadedMaster -Body $reloadedBody
Assert-True (Test-Path -LiteralPath (Get-SvPath backup)) 'second write leaves a backup'
Assert-True (@(Get-ChildItem -LiteralPath (Get-SvPath vaultdir) -Filter '*.tmp').Count -eq 0) 'no temp files left behind'

# --- session cache -----------------------------------------------------------
Write-Host '  session' -ForegroundColor Cyan
Clear-SvSession
Assert-True ($null -eq (Get-SvCachedKey -Kind master -Vault $reloaded)) 'no cached key to start'

Set-SvCachedKey -Kind master -Key $reloadedMaster -Vault $reloaded
Assert-True (Test-SvBytesEqual (Get-SvCachedKey -Kind master -Vault $reloaded) $reloadedMaster) 'dpapi round-trips the cached key'
Assert-True ($null -eq (Get-SvCachedKey -Kind data -Vault $reloaded)) 'caching master does not cache data'
Assert-True ($null -ne (Get-SvCachedKeyExpiry -Kind master -Vault $reloaded)) 'cached key reports an expiry'

$session = Read-SvSessionFile
$session.master.expires = [DateTimeOffset]::UtcNow.AddMinutes(-1).ToUnixTimeSeconds()
Write-SvSessionFile -Session $session
Assert-True ($null -eq (Get-SvCachedKey -Kind master -Vault $reloaded)) 'an expired cached key is refused'

Set-SvCachedKey -Kind master -Key $reloadedMaster -Vault $reloaded
$otherVault = @{ kdf = @{ sm = [Convert]::ToBase64String((New-SvRandomBytes 16)) } }
Assert-True ($null -eq (Get-SvCachedKey -Kind master -Vault $otherVault)) 'a cached key from another vault is refused'

Set-SvCachedKey -Kind master -Key $reloadedMaster -Vault $reloaded
Clear-SvSession
Assert-True (-not (Test-Path -LiteralPath (Get-SvPath session))) 'lock removes the session file'

# --- lockout -----------------------------------------------------------------
Write-Host '  lockout' -ForegroundColor Cyan
Clear-SvFailures
Assert-True ($null -eq (Get-SvLockoutRemaining)) 'no lockout to start'
Assert-True ((Read-SvLockout).fails -eq 0) 'failure counter starts at zero'

Write-SvLockout -State @{ fails = ($script:SV.MaxFails - 1); until = $null }
Assert-Throws { Add-SvFailure -What 'test password' } 'the final failure throws and locks'
Assert-True ($null -ne (Get-SvLockoutRemaining)) 'lockout is now active'
Assert-Throws { Assert-SvNotLockedOut } 'an active lockout blocks every command'
Assert-True (-not (Test-Path -LiteralPath (Get-SvPath session))) 'locking out drops cached keys'

Clear-SvFailures
Assert-True ($null -eq (Get-SvLockoutRemaining)) 'a successful password clears the lockout'

# --- selector scope ----------------------------------------------------------
# The installed `secure-vault` profile wrapper runs the CLI from inside a
# function, so src/ lands in a script scope that is NOT global. A scriptblock
# rehosted by GetNewClosure gets its own module scope, which cannot see those
# functions -- pressing Enter on a row died with "Unprotect-SvValue is not
# recognized". These two checks fail if that pattern ever comes back.
Write-Host '  selector scope' -ForegroundColor Cyan

Assert-True (-not (Select-String -Path (Join-Path $PSScriptRoot '..\src\*.ps1'), (Join-Path $PSScriptRoot '..\src\commands\*.ps1') -Pattern '\.GetNewClosure\(' -Quiet)) 'no scriptblock in src/ is rehosted by GetNewClosure'
Assert-True ((Get-Command Show-SvRevealTable).Parameters.ContainsKey('DataKey')) 'the selector takes the data key as an argument, not a capture'

# End to end, in the scope shape that broke: load inside a nested scope, build
# the reveal block inside a function, then decrypt through it.
$probeFile = Join-Path $env:SECUREVAULT_DIR 'scope-probe.ps1'
Set-Content -LiteralPath $probeFile -Encoding utf8 -Value @'
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
& {
    . $env:SV_LOAD
    $key = New-SvRandomBytes 32
    $entry = New-SvEntry -Type kv -Name 'SCOPE' -Project 'default'
    $entry.val = Protect-SvValue -DataKey $key -Entry $entry -Plaintext 'revealed'
    function Build-Reveal { { param($e, $k) Unprotect-SvValue -DataKey $k -Entry $e } }
    & (Build-Reveal) $entry $key
}
'@
$env:SV_LOAD = (Resolve-Path (Join-Path $PSScriptRoot '..\src\load.ps1')).Path
$probeOut = & (Get-Process -Id $PID).Path -NoProfile -Command "& { & '$probeFile' }" 2>&1
Assert-True (($probeOut -join '') -eq 'revealed') 'the reveal block decrypts when the CLI is invoked from a profile function'

# --- teardown ----------------------------------------------------------------
Remove-Item -LiteralPath $env:SECUREVAULT_DIR -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
if ($script:failed -eq 0) {
    Write-Host "  $script:passed passed, 0 failed" -ForegroundColor Green
    Write-Host ''
    exit 0
}
Write-Host "  $script:passed passed, $script:failed FAILED" -ForegroundColor Red
Write-Host ''
exit 1
