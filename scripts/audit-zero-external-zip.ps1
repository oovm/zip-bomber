# Living 13 stage Z: workspace must not depend on external `zip` or `flate2`.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

$fail = $false

foreach ($crate in @('flate2', 'zip')) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = cargo tree -i $crate 2>&1 | Out-String
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -eq 0) {
        Write-Error "forbidden dependency ``$crate`` is present in the crate graph:`n$output"
        $fail = $true
    }
}

$pattern = '(?i)\bflate2\b|(?m)^zip\s*='
$hits = Get-ChildItem -Recurse -Include '*.toml','*.rs' -File |
    Where-Object { $_.FullName -notmatch '\\target\\' } |
    Select-String -Pattern $pattern |
    ForEach-Object { "$($_.Path):$($_.LineNumber):$($_.Line.Trim())" }

if ($hits) {
    Write-Error "source or manifest still mentions forbidden crates:`n$($hits -join "`n")"
    $fail = $true
}

if ($fail) { exit 1 }
Write-Host 'audit passed: no external zip or flate2 dependency'
