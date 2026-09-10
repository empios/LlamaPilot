param([string]$Tag = "")
$ErrorActionPreference = "Stop"
node (Join-Path $PSScriptRoot "check-version-sync.mjs") $Tag
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
