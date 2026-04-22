param(
  [string]$GodotExecutable = ".\.tools\godot\editor\Godot_v4.6.2-stable_win64_console.exe"
)

$projectRoot = Split-Path -Parent $PSScriptRoot
$resolvedGodot = Resolve-Path -LiteralPath $GodotExecutable -ErrorAction Stop
$buildDir = Join-Path $projectRoot "build"
$outputPath = Join-Path $buildDir "HardwoodDynastyAlpha.exe"

New-Item -ItemType Directory -Force $buildDir | Out-Null

Push-Location $projectRoot
try {
  & $resolvedGodot --headless --path $projectRoot --export-release "Windows Desktop" $outputPath
  if ($LASTEXITCODE -ne 0) {
    throw "Godot export failed with exit code $LASTEXITCODE."
  }
}
finally {
  Pop-Location
}

Write-Output "Export complete: $outputPath"
