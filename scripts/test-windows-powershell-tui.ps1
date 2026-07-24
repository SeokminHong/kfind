[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $KfindPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$resolvedKfindPath = (Resolve-Path -LiteralPath $KfindPath).Path
$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) (
    'kfind-powershell-tui-' + [System.IO.Path]::GetRandomFileName()
)
$fixturePath = Join-Path $temporaryDirectory 'fixture.txt'

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null
    $lines = 1..200 | ForEach-Object { "needle result line $_" }
    [System.IO.File]::WriteAllLines(
        $fixturePath,
        $lines,
        [System.Text.UTF8Encoding]::new($false)
    )

    & $resolvedKfindPath --literal needle $fixturePath
    if ($LASTEXITCODE -ne 0) {
        throw "kfind TUI exited with code $LASTEXITCODE."
    }
} finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}

Write-Output 'PowerShell TUI smoke: ok'
