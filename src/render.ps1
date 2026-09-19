# render.ps1 — all console output. Nothing here decrypts or stores anything.

Set-StrictMode -Version Latest

function Write-SvInfo { param([string]$Message) Write-Host "  $Message" }
function Write-SvDim { param([string]$Message) Write-Host "  $Message" -ForegroundColor DarkGray }
function Write-SvOk { param([string]$Message) Write-Host "  $Message" -ForegroundColor Green }
function Write-SvWarn { param([string]$Message) Write-Host "  $Message" -ForegroundColor Yellow }
function Write-SvError { param([string]$Message) Write-Host "  $Message" -ForegroundColor Red }

function Write-SvBanner {
    param([Parameter(Mandatory)][string]$State, [string]$Detail = '')

    $color = switch ($State) {
        'UNLOCKED' { 'Green' }
        'OPEN' { 'Cyan' }
        default { 'DarkGray' }
    }
    Write-Host ''
    Write-Host "  $($script:SV.Name) " -NoNewline
    Write-Host $State -ForegroundColor $color -NoNewline
    if ($Detail) { Write-Host "  $Detail" -ForegroundColor DarkGray } else { Write-Host '' }
    Write-Host ''
}

# Masks are derived from the CIPHERTEXT, never the plaintext. AES-GCM ciphertext
# is exactly as long as its plaintext, so the length is already public to anyone
# holding the file — but this means drawing the list decrypts nothing at all.
# No characters of the value are shown either: four leaked characters across
# every row is a real loss when the whole point is that nothing is on screen.
function Get-SvMaskedEntry {
    param([Parameter(Mandatory)][hashtable]$Entry)

    if ($Entry.type -eq 'codes') {
        $used = @($Entry.used).Count
        $total = $Entry['total']
        if ($total) { return '{0} codes, {1} unused' -f $total, ($total - $used) }
        return "backup codes, $used used"
    }

    $length = Get-SvSealedSize $Entry.val
    if ($length -le 0) { return '(empty)' }
    [string]$script:SV.MaskChar * [Math]::Min($script:SV.MaskMaxDots, $length)
}

# Backup codes are a multi-line payload; the tables are single-line, so codes
# collapse to one selectable row and the numbered list belongs to --show.
function Get-SvDisplayValue {
    param([Parameter(Mandatory)][hashtable]$Entry, [Parameter(Mandatory)][AllowEmptyString()][string]$Payload)

    if ($Entry.type -ne 'codes') { return $Payload }
    (Split-SvCodes $Payload) -join '  '
}

function Get-SvShortCipher {
    param([Parameter(Mandatory)]$Sealed, [int]$Width = 34)

    $text = $Sealed.c
    if ($text.Length -le $Width) { return $text }
    '{0}...{1}' -f $text.Substring(0, $Width - 8), $text.Substring($text.Length - 5)
}

function Format-SvRow {
    param($Index, [string]$Type, [string]$Name, [string]$Value, [int]$NameWidth = 22)

    '{0,3}  {1,-5}  {2}  {3}' -f $Index, $Type, $Name.PadRight($NameWidth), $Value
}

function Get-SvNameWidth {
    param([array]$Entries)

    $longest = 4
    foreach ($entry in $Entries) {
        if ($entry.name.Length -gt $longest) { $longest = $entry.name.Length }
    }
    [Math]::Min(40, $longest)
}

# The locked view: entry names are readable (master layer is open), every value
# is still nothing but authenticated ciphertext.
function Show-SvCipherTable {
    param([Parameter(Mandatory)][array]$Entries)

    if ($Entries.Count -eq 0) {
        Write-SvDim 'Vault is empty. Add something with:  secure-vault --add'
        return
    }

    $width = Get-SvNameWidth $Entries
    Write-Host (Format-SvRow -Index '#' -Type 'TYPE' -Name 'NAME' -Value 'CIPHERTEXT (aes-256-gcm)' -NameWidth $width) -ForegroundColor DarkGray
    for ($i = 0; $i -lt $Entries.Count; $i++) {
        $entry = $Entries[$i]
        $cipher = Get-SvShortCipher $entry.val
        $size = Get-SvSealedSize $entry.val
        Write-Host (Format-SvRow -Index ($i + 1) -Type $entry.type -Name $entry.name -Value "$cipher  ($size B)" -NameWidth $width)
    }
    Write-Host ''
    Write-SvDim 'run  secure-vault --unlock  to decrypt these values'
}

function Show-SvPlainTable {
    param([Parameter(Mandatory)][array]$Entries, [Parameter(Mandatory)][hashtable]$Values)

    $width = Get-SvNameWidth $Entries
    Write-Host (Format-SvRow -Index '#' -Type 'TYPE' -Name 'NAME' -Value 'VALUE' -NameWidth $width) -ForegroundColor DarkGray
    for ($i = 0; $i -lt $Entries.Count; $i++) {
        $entry = $Entries[$i]
        Write-Host (Format-SvRow -Index ($i + 1) -Type $entry.type -Name $entry.name -Value $Values[$entry.id] -NameWidth $width)
    }
    Write-Host ''
}

function Show-SvCodes {
    param([Parameter(Mandatory)][hashtable]$Entry, [Parameter(Mandatory)][string[]]$Codes)

    $used = @($Entry.used)
    for ($i = 0; $i -lt $Codes.Count; $i++) {
        $spent = $used -contains $i
        $label = '{0,4}. {1}' -f ($i + 1), $Codes[$i]
        if ($spent) { Write-Host "  $label  used" -ForegroundColor DarkGray }
        else { Write-Host "  $label" }
    }
    Write-Host ''
    Write-SvDim "$($Codes.Count - $used.Count) of $($Codes.Count) codes unused"
}

function Show-SvProjectTable {
    param([Parameter(Mandatory)][array]$Projects)

    Write-Host ('     {0,-24}  {1}' -f 'PROJECT', 'ENTRIES') -ForegroundColor DarkGray
    foreach ($project in $Projects) {
        $marker = if ($project.current) { ' >' } else { '  ' }
        $line = '{0}   {1,-24}  {2}' -f $marker, $project.name, $project.entries
        if ($project.current) { Write-Host $line -ForegroundColor Green }
        else { Write-Host $line }
    }
    Write-Host ''
    Write-SvDim 'switch with:  secure-vault --project NAME'
}

function Show-SvProjectChoices {
    param([Parameter(Mandatory)][array]$Choices)

    for ($i = 0; $i -lt $Choices.Count; $i++) {
        Write-Host ('  {0,3}. {1}' -f ($i + 1), $Choices[$i])
    }
    Write-Host ''
}

function Show-SvHelp {
    @"

  $($script:SV.Name) — local encrypted vault · aes-256-gcm · pbkdf2-sha256 · no network, ever

  secure-vault                open the vault (master password) and print it still encrypted
  secure-vault --unlock       decrypt values (decrypt password), then reveal on select
  secure-vault --lock         forget both cached keys right now
  secure-vault --status       show state, TTL and lockout without prompting for anything

  secure-vault --show NAME    print one decrypted value in full
  secure-vault --all          print every decrypted value, no masking (for piping)
  secure-vault --use NAME     reveal and burn the next unused backup code

  secure-vault --add          add an entry: kv | codes | pw
  secure-vault --import       bulk paste  key1,value1; key2,value2;  as kv entries
  secure-vault --move NAME    move an entry to another project
  secure-vault --rm NAME      delete an entry

  secure-vault --projects     list projects and their entry counts
  secure-vault --project NAME switch to a project (creates it if new)
  secure-vault --rm-project NAME   delete a project and everything in it

  secure-vault --init         create a new vault (sets all three passwords)
  secure-vault --rotate       fresh salts + nonces, optionally new passwords
  secure-vault --emergency    erase the vault with the deletion password (--force skips confirm)
  secure-vault --help         this

  vault   $(Get-SvPath vault)
  keys    master opens the file · decrypt reveals the values · deletion destroys everything

"@ | Write-Host
}
