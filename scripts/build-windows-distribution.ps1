[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Version,

    [Parameter(Mandatory = $true)]
    [string] $Binary,

    [Parameter(Mandatory = $true)]
    [string] $FullPosDirectory,

    [Parameter(Mandatory = $true)]
    [string] $ComponentDirectory,

    [Parameter(Mandatory = $true)]
    [string] $OutputArchive
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$') {
    throw "Invalid release version: $Version"
}

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$binaryPath = [System.IO.Path]::GetFullPath($Binary)
$fullPosPath = [System.IO.Path]::GetFullPath($FullPosDirectory)
$componentPath = [System.IO.Path]::GetFullPath($ComponentDirectory)
$archivePath = [System.IO.Path]::GetFullPath($OutputArchive)
$archiveDirectory = Split-Path -Parent $archivePath
$stage = Join-Path $archiveDirectory ('.kfind-windows-' + [System.IO.Path]::GetRandomFileName())

function Copy-RequiredFile {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Source,

        [Parameter(Mandatory = $true)]
        [string] $Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Required distribution file is missing: $Source"
    }
    Copy-Item -LiteralPath $Source -Destination $Destination
}

New-Item -ItemType Directory -Path $archiveDirectory -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $stage 'bin') -Force | Out-Null
$dataDirectory = Join-Path $stage 'share\kfind'
$licenseDirectory = Join-Path $stage 'share\doc\kfind\LICENSES'
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $licenseDirectory -Force | Out-Null

try {
    $stagedBinary = Join-Path $stage 'bin\kfind.exe'
    Copy-RequiredFile -Source $binaryPath -Destination $stagedBinary
    Copy-RequiredFile `
        -Source (Join-Path $fullPosPath 'lexicon.bin') `
        -Destination (Join-Path $dataDirectory 'lexicon.bin')
    Copy-RequiredFile `
        -Source (Join-Path $fullPosPath 'MANIFEST.toml') `
        -Destination (Join-Path $dataDirectory 'full-pos-MANIFEST.toml')
    Copy-RequiredFile `
        -Source (Join-Path $fullPosPath 'STATS.toml') `
        -Destination (Join-Path $dataDirectory 'full-pos-STATS.toml')
    Copy-RequiredFile `
        -Source (Join-Path $componentPath 'morphology-component-compact.kfc') `
        -Destination (Join-Path $dataDirectory 'morphology-component-compact.kfc')
    Copy-RequiredFile `
        -Source (Join-Path $componentPath 'MANIFEST.toml') `
        -Destination (Join-Path $dataDirectory 'component-MANIFEST.toml')
    Copy-RequiredFile `
        -Source (Join-Path $repositoryRoot 'data\enriched\predicates.tsv') `
        -Destination (Join-Path $dataDirectory 'predicates.enriched.tsv')
    Copy-RequiredFile `
        -Source (Join-Path $repositoryRoot 'data\enriched\MANIFEST.toml') `
        -Destination (Join-Path $dataDirectory 'predicates.enriched.MANIFEST.toml')
    Copy-RequiredFile `
        -Source (Join-Path $repositoryRoot 'LICENSE') `
        -Destination (Join-Path $licenseDirectory 'kfind-MIT.txt')
    Copy-RequiredFile `
        -Source (Join-Path $fullPosPath 'LICENSES\mecab-ko-dic-COPYING') `
        -Destination (Join-Path $licenseDirectory 'mecab-ko-dic-COPYING')
    Copy-RequiredFile `
        -Source (Join-Path $repositoryRoot 'data\enriched\NOTICE.md') `
        -Destination (Join-Path $licenseDirectory 'enriched-predicates-NOTICE.md')

    $versionOutput = (& $stagedBinary --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $versionOutput -ne "kfind $Version") {
        throw "Unexpected Windows binary version: $versionOutput"
    }

    $dataCheckOutput = (& $stagedBinary --check-data --json --data-dir $dataDirectory | Out-String)
    if ($LASTEXITCODE -ne 0) {
        throw "Windows distribution data check failed with exit code $LASTEXITCODE."
    }
    $dataCheck = $dataCheckOutput | ConvertFrom-Json
    if ($dataCheck.status -ne 'ok' -or $dataCheck.kfind_version -ne $Version) {
        throw "Windows distribution data check returned unexpected package metadata."
    }
    if ($dataCheck.component.resource_version -ne $Version) {
        throw "Windows distribution component version does not match $Version."
    }

    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    Compress-Archive `
        -Path (Join-Path $stage '*') `
        -DestinationPath $archivePath `
        -CompressionLevel Optimal
} finally {
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
}

Write-Output $archivePath
