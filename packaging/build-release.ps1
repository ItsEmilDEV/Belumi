# Builds the Windows release: dist\belumi-<version>-windows-x64.zip (portable) and, when Inno Setup is installed,
# dist\belumi-<version>-windows-x64-setup.exe. Usage: pwsh packaging\build-release.ps1 [version]
# Env:   BELUMI_FEATURES  client cargo features (default: belumi, which brings plugins and voice chat along)
param([string]$Version = "")
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

if (-not $Version) {
    $Version = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
}
# The programs carry the C runtime with them, so a clean Windows needs no Visual C++ redistributable.
$env:RUSTFLAGS = "$env:RUSTFLAGS -C target-feature=+crt-static".Trim()
$features = if ($null -ne $env:BELUMI_FEATURES) { $env:BELUMI_FEATURES } else { "belumi" }
$name = "belumi-$Version-windows-x64"
$stage = Join-Path "dist" $name

Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stage | Out-Null

cargo build --release -p belumi-launcher
if ($LASTEXITCODE -ne 0) { throw "launcher build failed" }
Push-Location "vendor/horizon"
if ($features) { cargo build --release -p horizon-client --features $features } else { cargo build --release -p horizon-client }
if ($LASTEXITCODE -ne 0) { throw "client build failed" }
Pop-Location

Copy-Item "target/release/belumi-launcher.exe" $stage
Copy-Item "vendor/horizon/target/release/horizon-client.exe" $stage
Copy-Item "LICENSE" $stage
Copy-Item "packaging/README-release-windows.txt" (Join-Path $stage "README.txt")

$zip = "dist/$name.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip
(Get-FileHash $zip -Algorithm SHA256).Hash.ToLower() + "  $name.zip" | Set-Content "dist/$name.zip.sha256"
Write-Host $zip

$iscc = Get-Command iscc -ErrorAction SilentlyContinue
if (-not $iscc) {
    $candidate = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    if (Test-Path $candidate) { $iscc = $candidate }
}
if ($iscc) {
    & $iscc "/DAppVersion=$Version" "/DSourceDir=$((Resolve-Path $stage).Path)" "/DOutputDir=$((Resolve-Path dist).Path)" "/DOutputName=$name-setup" "packaging/belumi.iss"
    if ($LASTEXITCODE -ne 0) { throw "installer build failed" }
    (Get-FileHash "dist/$name-setup.exe" -Algorithm SHA256).Hash.ToLower() + "  $name-setup.exe" | Set-Content "dist/$name-setup.exe.sha256"
    Write-Host "dist/$name-setup.exe"
} elseif ($env:CI) {
    throw "Inno Setup not found: the installer cannot be built"
} else {
    Write-Host "Inno Setup not found: only the portable zip was built."
}
