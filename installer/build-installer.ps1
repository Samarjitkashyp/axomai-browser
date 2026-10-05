# Builds Axomai-Setup-<version>.exe (plus a .sha256 file) from a release build. It needs nothing but Windows itself:
# the setup program is C# (Setup.cs) compiled with the compiler that ships with .NET Framework, with the application
# files embedded as a zip.
#   cargo build --release            (in rust_desktop)
#   powershell -File installer\build-installer.ps1 -Exe <path to axomai_browser.exe> -Out <folder>
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Out = "$PSScriptRoot\..\dist",
    [string]$Version = ""
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot\..").Path
if (-not $Version) {
    $m = Select-String -Path (Join-Path $root 'rust_desktop\Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    $Version = $m.Matches[0].Groups[1].Value
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$stage = Join-Path ([IO.Path]::GetTempPath()) "axomai-stage-$Version"
$app = Join-Path $stage 'app'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $app | Out-Null

Copy-Item -LiteralPath $Exe -Destination (Join-Path $app 'axomai_browser.exe')
Copy-Item -Recurse -Path (Join-Path $root 'ui') -Destination (Join-Path $app 'ui')
Copy-Item -LiteralPath "$PSScriptRoot\uninstall.ps1" -Destination (Join-Path $app 'uninstall.ps1')
Set-Content -LiteralPath (Join-Path $app 'version.txt') -Value $Version -NoNewline

$payload = Join-Path $stage 'payload.zip'
Compress-Archive -Path (Join-Path $app '*') -DestinationPath $payload -CompressionLevel Optimal

$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path $csc)) { $csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework\v4.0.30319\csc.exe' }
if (-not (Test-Path $csc)) { throw 'The .NET Framework C# compiler (csc.exe) was not found.' }

$target = Join-Path $Out "Axomai-Setup-$Version.exe"
if (Test-Path $target) { Remove-Item -Force $target }
$cscArgs = @('/nologo', '/target:winexe', '/optimize+', "/out:$target",
    '/r:System.Windows.Forms.dll', '/r:System.IO.Compression.dll', '/r:System.IO.Compression.FileSystem.dll',
    "/resource:$payload,payload.zip", "$PSScriptRoot\Setup.cs")
& $csc @cscArgs
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $target)) { throw "csc failed (exit code $LASTEXITCODE)" }

$hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLower()
Set-Content -LiteralPath "$target.sha256" -Value "$hash  $(Split-Path $target -Leaf)" -Encoding ASCII
Write-Output "Built $target ($([math]::Round((Get-Item $target).Length / 1MB, 1)) MB)"
Write-Output "SHA-256 $hash"
