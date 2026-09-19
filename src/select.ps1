# select.ps1 — reveal-on-focus. A terminal cannot see mouse clicks without VT
# mouse tracking, so arrow keys + Enter are the honest equivalent.
#
# Only the focused row can ever be revealed, and moving off it hides it again:
# at most one secret is on screen at any moment, and it disappears the instant
# your attention moves.

Set-StrictMode -Version Latest

function Write-SvPadded {
    param([string]$Text, [string]$Foreground, [string]$Background)

    $width = [Math]::Max(20, [Console]::WindowWidth - 1)
    if ($Text.Length -gt $width) { $Text = $Text.Substring(0, $width) }
    $line = $Text.PadRight($width)

    $hostArgs = @{ Object = $line }
    if ($Foreground) { $hostArgs.ForegroundColor = $Foreground }
    if ($Background) { $hostArgs.BackgroundColor = $Background }
    Write-Host @hostArgs
}

function Test-SvCanSelect {
    param([Parameter(Mandatory)][int]$RowCount)

    -not [Console]::IsInputRedirected -and
    -not [Console]::IsOutputRedirected -and
    ($RowCount + 6) -lt [Console]::WindowHeight
}

# Returns $null, or @{ Action = 'move'|'delete'; Name = '<entry name>' } for the
# caller to run once the interactive loop has torn down.
function Show-SvRevealTable {
    param(
        [Parameter(Mandatory)][array]$Entries,
        [Parameter(Mandatory)][scriptblock]$Reveal,
        [Parameter(Mandatory)][byte[]]$DataKey
    )

    if ($Entries.Count -eq 0) {
        Write-SvDim 'No entries in this project. Add one with:  secure-vault --add'
        return $null
    }

    # Masks come from ciphertext length, so building this table decrypts nothing.
    $masked = @{}
    foreach ($entry in $Entries) { $masked[$entry.id] = Get-SvMaskedEntry -Entry $entry }

    if (-not (Test-SvCanSelect -RowCount $Entries.Count)) {
        Show-SvPlainTable -Entries $Entries -Values $masked
        Write-SvDim 'reveal one with:  secure-vault --show NAME'
        return $null
    }

    # Exactly one plaintext exists at a time, produced on demand by $Reveal and
    # dropped the moment you hide it or move off the row.
    $width = Get-SvNameWidth $Entries
    $cursor = 0
    $shownValue = $null
    $top = [Console]::CursorTop
    $wasVisible = [Console]::CursorVisible
    [Console]::CursorVisible = $false

    try {
        while ($true) {
            [Console]::SetCursorPosition(0, $top)
            Write-SvPadded -Text ('  ' + (Format-SvRow -Index '#' -Type 'TYPE' -Name 'NAME' -Value 'VALUE' -NameWidth $width)) -Foreground DarkGray

            for ($i = 0; $i -lt $Entries.Count; $i++) {
                $entry = $Entries[$i]
                $isFocused = ($i -eq $cursor)
                $isRevealed = ($isFocused -and $null -ne $shownValue)
                $shown = if ($isRevealed) { $shownValue } else { $masked[$entry.id] }

                $row = '  ' + (Format-SvRow -Index ($i + 1) -Type $entry.type -Name $entry.name -Value $shown -NameWidth $width)
                if ($isRevealed) { Write-SvPadded -Text $row -Foreground Black -Background Green }
                elseif ($isFocused) { Write-SvPadded -Text $row -Foreground Black -Background Gray }
                else { Write-SvPadded -Text $row }
            }

            Write-SvPadded -Text ''
            Write-SvPadded -Text '  up/down move   Enter show/hide   M move to project   D delete   Esc done' -Foreground DarkGray

            $key = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
            switch ($key.VirtualKeyCode) {
                38 { $cursor = [Math]::Max(0, $cursor - 1); $shownValue = $null }                  # up
                40 { $cursor = [Math]::Min($Entries.Count - 1, $cursor + 1); $shownValue = $null }  # down
                36 { $cursor = 0; $shownValue = $null }                                            # home
                35 { $cursor = $Entries.Count - 1; $shownValue = $null }                           # end
                13 {
                    # decrypt on demand, drop on hide
                    if ($null -ne $shownValue) { $shownValue = $null }
                    else { $shownValue = & $Reveal $Entries[$cursor] $DataKey }
                }
                77 { return @{ Action = 'move'; Name = $Entries[$cursor].name } }                 # m
                68 { return @{ Action = 'delete'; Name = $Entries[$cursor].name } }               # d
                27 { return $null }                                                               # esc
                81 { return $null }                                                               # q
                67 { if ($key.Character -eq [char]3) { return $null } }                           # ctrl+c
            }
        }
    }
    finally {
        $shownValue = $null
        [Console]::CursorVisible = $wasVisible
        Write-Host ''
    }
}
