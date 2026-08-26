param(
    [string]$Tag = ""
)

$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$packagePath = Join-Path $repositoryRoot "package.json"
$packageLockPath = Join-Path $repositoryRoot "package-lock.json"
$cargoPath = Join-Path $repositoryRoot "src-tauri/Cargo.toml"
$cargoLockPath = Join-Path $repositoryRoot "src-tauri/Cargo.lock"
$tauriPath = Join-Path $repositoryRoot "src-tauri/tauri.conf.json"

$package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
$packageLockJson = Get-Content -LiteralPath $packageLockPath -Raw
# Windows PowerShell 5.1 rejects JSON properties with an empty name. npm uses one for the root
# package, so give only that entry a temporary parseable name before deserializing the lockfile.
$packageLock = $packageLockJson.Replace('"": {', '"__rootPackage__": {') | ConvertFrom-Json
$tauri = Get-Content -LiteralPath $tauriPath -Raw | ConvertFrom-Json
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargoLock = Get-Content -LiteralPath $cargoLockPath -Raw

$cargoPackageVersion = [regex]::Match(
    $cargo,
    '(?ms)^\[package\]\s*.*?^version\s*=\s*"([^\"]+)"'
)
$cargoPackageName = [regex]::Match(
    $cargo,
    '(?ms)^\[package\]\s*.*?^name\s*=\s*"([^\"]+)"'
)
if (-not $cargoPackageVersion.Success -or -not $cargoPackageName.Success) {
    throw "Could not read the package version from src-tauri/Cargo.toml."
}

$cargoName = [string]$cargoPackageName.Groups[1].Value
$cargoLockPackage = [regex]::Match(
    $cargoLock,
    "(?ms)^\[\[package\]\]\s*\r?\nname\s*=\s*`"$([regex]::Escape($cargoName))`"\s*\r?\nversion\s*=\s*`"([^`"]+)`""
)
if (-not $cargoLockPackage.Success) {
    throw "Could not find the $cargoName package in src-tauri/Cargo.lock."
}

$lockRoot = $packageLock.packages.__rootPackage__
if ($null -eq $lockRoot) {
    throw "package-lock.json does not contain the root package entry."
}

$expectedVersion = [string]$package.version
$versions = [ordered]@{
    "package.json" = [string]$package.version
    "package-lock.json" = [string]$packageLock.version
    "package-lock.json root package" = [string]$lockRoot.version
    "src-tauri/Cargo.toml" = [string]$cargoPackageVersion.Groups[1].Value
    "src-tauri/Cargo.lock" = [string]$cargoLockPackage.Groups[1].Value
    "src-tauri/tauri.conf.json" = [string]$tauri.version
}

foreach ($entry in $versions.GetEnumerator()) {
    if ($entry.Value -ne $expectedVersion) {
        throw "$($entry.Key) declares version $($entry.Value), expected $expectedVersion."
    }
}

$expectedName = [string]$package.name
if (
    [string]$packageLock.name -ne $expectedName -or
    [string]$lockRoot.name -ne $expectedName -or
    $cargoName -ne $expectedName
) {
    throw "The npm manifests and Cargo.toml must declare the same package name."
}

if ($Tag -and $Tag -ne "v$expectedVersion") {
    throw "Tag $Tag does not match package version v$expectedVersion."
}

Write-Output "Version metadata is synchronized at $expectedVersion."
