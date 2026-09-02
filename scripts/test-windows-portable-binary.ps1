[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Binary
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$binaryPath = [System.IO.Path]::GetFullPath($Binary)
if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
    throw "Windows binary is missing: $binaryPath"
}

function Find-Dumpbin {
    $command = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        throw 'dumpbin.exe and vswhere.exe are unavailable.'
    }

    $installationPath = (
        & $vswhere `
            -latest `
            -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath |
            Select-Object -First 1
    )
    if ([string]::IsNullOrWhiteSpace($installationPath)) {
        throw 'Visual Studio C++ build tools are unavailable.'
    }

    $dumpbinCandidates = @(
        Get-ChildItem `
            -Path (Join-Path $installationPath 'VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe') `
            -File |
            Sort-Object FullName -Descending
    )
    if ($dumpbinCandidates.Count -eq 0) {
        throw 'dumpbin.exe was not found in the Visual Studio C++ build tools.'
    }
    return $dumpbinCandidates[0].FullName
}

$dumpbin = Find-Dumpbin
$dependencyOutput = (& $dumpbin /nologo /dependents $binaryPath | Out-String)
if ($LASTEXITCODE -ne 0) {
    throw "dumpbin.exe failed with exit code $LASTEXITCODE."
}

$imports = @(
    [regex]::Matches($dependencyOutput, '(?im)^\s+([A-Za-z0-9._-]+\.dll)\s*$') |
        ForEach-Object { $_.Groups[1].Value }
)
if ($imports.Count -eq 0) {
    throw 'No Windows DLL imports were found.'
}

$dynamicRuntimePatterns = @(
    '^VCRUNTIME[0-9]+(?:_[0-9]+)?\.dll$',
    '^MSVCP[0-9]+(?:_[0-9]+)?\.dll$',
    '^CONCRT[0-9]+\.dll$',
    '^api-ms-win-crt-.+\.dll$',
    '^ucrtbase\.dll$'
)
$dynamicRuntimeImports = @(
    $imports | Where-Object {
        $import = $_
        @($dynamicRuntimePatterns | Where-Object { $import -match $_ }).Count -gt 0
    }
)
if ($dynamicRuntimeImports.Count -gt 0) {
    throw "Windows binary requires dynamic C runtime DLLs: $($dynamicRuntimeImports -join ', ')"
}

Write-Output 'Windows portable binary dependencies: ok'
