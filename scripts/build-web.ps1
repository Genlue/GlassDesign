[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$OutDir,

    [switch]$KeepSymbols
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$out = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir)
$target = Join-Path $root 'target'
$bindgen = (Get-Command wasm-bindgen -CommandType Application).Source
$version = & $bindgen --version
if ($LASTEXITCODE -ne 0 -or $version -ne 'wasm-bindgen 0.2.130') {
    throw "Expected wasm-bindgen CLI 0.2.130 on PATH (found '$version')."
}

Push-Location -LiteralPath $root
try {
    & cargo build --package glass-web --target wasm32-unknown-unknown --release --locked --offline --target-dir $target
    if ($LASTEXITCODE -ne 0) { throw "glass-web cargo build failed ($LASTEXITCODE)." }
} finally {
    Pop-Location
}

[void][IO.Directory]::CreateDirectory($out)
$wasm = Join-Path $target 'wasm32-unknown-unknown/release/glass_web.wasm'
$bindgenArgs = @('--target', 'web', '--out-dir', $out, '--out-name', 'glass_web', '--remove-producers-section')
if (-not $KeepSymbols) { $bindgenArgs += '--remove-name-section' }
& $bindgen @bindgenArgs $wasm
if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed ($LASTEXITCODE)." }

Copy-Item -LiteralPath (Join-Path $root '控件/css/glass.css') -Destination (Join-Path $out 'glass.css')
Copy-Item -LiteralPath (Join-Path $root '控件/assets/glass-defs.svg') -Destination (Join-Path $out 'glass-defs.svg')
Write-Host "GlassDesign web assets: $out"
