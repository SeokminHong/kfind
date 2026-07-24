[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Version,

    [Parameter(Mandatory = $true)]
    [string] $ArchiveSha256,

    [Parameter(Mandatory = $true)]
    [string] $OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$') {
    throw "Invalid release version: $Version"
}
if ($ArchiveSha256 -notmatch '^[0-9a-fA-F]{64}$') {
    throw "Invalid SHA-256: $ArchiveSha256"
}

$templateDirectory = [System.IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot '../packaging/chocolatey')
)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$toolsPath = Join-Path $outputPath 'tools'
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
$replacements = @{
    '@VERSION@' = $Version
    '@ARCHIVE_SHA256@' = $ArchiveSha256.ToLowerInvariant()
}

function Write-RenderedTemplate {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Source,

        [Parameter(Mandatory = $true)]
        [string] $Destination
    )

    $content = [System.IO.File]::ReadAllText($Source)
    foreach ($replacement in $replacements.GetEnumerator()) {
        $content = $content.Replace($replacement.Key, $replacement.Value)
    }
    foreach ($placeholder in $replacements.Keys) {
        if ($content.Contains($placeholder)) {
            throw "Unresolved placeholder $placeholder in $Source"
        }
    }
    [System.IO.File]::WriteAllText($Destination, $content, $utf8WithoutBom)
}

New-Item -ItemType Directory -Path $toolsPath -Force | Out-Null
Write-RenderedTemplate `
    -Source (Join-Path $templateDirectory 'kfind.nuspec.in') `
    -Destination (Join-Path $outputPath 'kfind.nuspec')
Write-RenderedTemplate `
    -Source (Join-Path (Join-Path $templateDirectory 'tools') 'chocolateyInstall.ps1.in') `
    -Destination (Join-Path $toolsPath 'chocolateyInstall.ps1')
Write-RenderedTemplate `
    -Source (Join-Path (Join-Path $templateDirectory 'tools') 'VERIFICATION.txt.in') `
    -Destination (Join-Path $toolsPath 'VERIFICATION.txt')

Write-Output (Join-Path $outputPath 'kfind.nuspec')
