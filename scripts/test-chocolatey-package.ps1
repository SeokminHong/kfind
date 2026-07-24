[CmdletBinding()]
param(
    [switch] $SkipPack
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) (
    'kfind-chocolatey-' + [System.IO.Path]::GetRandomFileName()
)
$packageDirectory = Join-Path $temporaryDirectory 'package'
$nupkgDirectory = Join-Path $temporaryDirectory 'nupkg'
$version = '1.2.3-rc.4'
$checksum = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'

function Assert-True {
    param(
        [Parameter(Mandatory = $true)]
        [bool] $Condition,

        [Parameter(Mandatory = $true)]
        [string] $Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null
    & (Join-Path (Join-Path $repositoryRoot 'scripts') 'render-chocolatey-package.ps1') `
        -Version $version `
        -ArchiveSha256 $checksum `
        -OutputDirectory $packageDirectory | Out-Null

    $nuspecPath = Join-Path $packageDirectory 'kfind.nuspec'
    $toolsDirectory = Join-Path $packageDirectory 'tools'
    $installPath = Join-Path $toolsDirectory 'chocolateyInstall.ps1'
    $verificationPath = Join-Path $toolsDirectory 'VERIFICATION.txt'
    Assert-True (Test-Path -LiteralPath $nuspecPath -PathType Leaf) 'Rendered nuspec is missing.'
    Assert-True (Test-Path -LiteralPath $installPath -PathType Leaf) 'Rendered install script is missing.'
    Assert-True (Test-Path -LiteralPath $verificationPath -PathType Leaf) 'Rendered verification file is missing.'

    $rendered = (
        [System.IO.File]::ReadAllText($nuspecPath) +
        [System.IO.File]::ReadAllText($installPath) +
        [System.IO.File]::ReadAllText($verificationPath)
    )
    Assert-True (-not $rendered.Contains('@VERSION@')) 'Version placeholder was not replaced.'
    Assert-True (-not $rendered.Contains('@ARCHIVE_SHA256@')) 'Checksum placeholder was not replaced.'
    Assert-True ($rendered.Contains("v$version")) 'Rendered package does not use the release tag.'
    Assert-True ($rendered.Contains($checksum)) 'Rendered package does not use the archive checksum.'

    [xml] $nuspec = [System.IO.File]::ReadAllText($nuspecPath)
    Assert-True ($nuspec.package.metadata.id -eq 'kfind') 'Unexpected Chocolatey package ID.'
    Assert-True ($nuspec.package.metadata.version -eq $version) 'Unexpected Chocolatey package version.'

    $tokens = $null
    $parseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        $installPath,
        [ref] $tokens,
        [ref] $parseErrors
    ) | Out-Null
    Assert-True (@($parseErrors).Count -eq 0) 'Chocolatey install script has PowerShell parse errors.'

    foreach ($path in @($nuspecPath, $installPath, $verificationPath)) {
        $bytes = [System.IO.File]::ReadAllBytes($path)
        $hasUtf8Bom = $bytes.Length -ge 3 `
            -and $bytes[0] -eq 0xEF `
            -and $bytes[1] -eq 0xBB `
            -and $bytes[2] -eq 0xBF
        Assert-True (-not $hasUtf8Bom) "Rendered file has a UTF-8 BOM: $path"
    }

    if (-not $SkipPack) {
        New-Item -ItemType Directory -Path $nupkgDirectory -Force | Out-Null
        & choco pack $nuspecPath --output-directory $nupkgDirectory --limit-output
        if ($LASTEXITCODE -ne 0) {
            throw "choco pack failed with exit code $LASTEXITCODE."
        }
        $packages = @(Get-ChildItem -LiteralPath $nupkgDirectory -Filter '*.nupkg')
        Assert-True ($packages.Count -eq 1) 'choco pack did not create exactly one package.'
        Assert-True ($packages[0].Name -eq "kfind.$version.nupkg") 'Unexpected Chocolatey package filename.'
    }
} finally {
    if (Test-Path -LiteralPath $temporaryDirectory) {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force
    }
}

Write-Output 'Chocolatey package template: ok'
