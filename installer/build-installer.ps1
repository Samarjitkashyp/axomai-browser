# Builds Axomai-Setup-<version>.exe (plus a .sha256 file) from a release build, using IExpress, which ships with Windows.
#   cargo build --release            (in rust_desktop)
#   powershell -File installer\build-installer.ps1 -Exe <path to axomai_browser.exe> -Out <folder>
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Out = "$PSScriptRoot\..\dist",
    [string]$Version = ""
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\.."
if (-not $Version) {
    $m = Select-String -Path (Join-Path $root 'rust_desktop\Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    $Version = $m.Matches[0].Groups[1].Value
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$stage = Join-Path ([IO.Path]::GetTempPath()) "axomai-stage-$Version"
$app = Join-Path $stage 'app'
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $app | Out-Null

Copy-Item -LiteralPath $Exe -Destination (Join-Path $app 'axomai_browser.exe')
Copy-Item -Recurse -Path (Join-Path $root 'ui') -Destination (Join-Path $app 'ui')
Copy-Item -LiteralPath "$PSScriptRoot\uninstall.ps1" -Destination (Join-Path $app 'uninstall.ps1')
Set-Content -LiteralPath (Join-Path $app 'version.txt') -Value $Version -NoNewline

$payload = Join-Path $stage 'payload.zip'
Compress-Archive -Path (Join-Path $app '*') -DestinationPath $payload -CompressionLevel Optimal
Copy-Item -LiteralPath "$PSScriptRoot\install.ps1" -Destination (Join-Path $stage 'install.ps1')

$target = Join-Path $Out "Axomai-Setup-$Version.exe"
Remove-Item -Force $target -ErrorAction SilentlyContinue
$run = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File install.ps1'
$sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3
[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=0
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=
TargetName=$target
FriendlyName=Axomai Browser Setup
AppLaunched=$run
PostInstallCmd=<None>
AdminQuietInstCmd=$run -Quiet -Launch
UserQuietInstCmd=$run -Quiet -Launch
SourceFiles=SourceFiles
[Strings]
FILE0="install.ps1"
FILE1="payload.zip"
[SourceFiles]
SourceFiles0=$stage\
[SourceFiles0]
%FILE0%=
%FILE1%=
"@
$sedFile = Join-Path $stage 'axomai.sed'
Set-Content -LiteralPath $sedFile -Value $sed -Encoding ASCII
$p = Start-Process -FilePath "$env:WINDIR\System32\iexpress.exe" -ArgumentList "/N /Q `"$sedFile`"" -Wait -PassThru -WindowStyle Hidden
if (-not (Test-Path -LiteralPath $target)) { throw "IExpress did not produce $target (exit code $($p.ExitCode))" }
$hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLower()
Set-Content -LiteralPath "$target.sha256" -Value "$hash  $(Split-Path $target -Leaf)" -Encoding ASCII
Write-Output "Built $target ($([math]::Round((Get-Item $target).Length / 1MB, 1)) MB)"
Write-Output "SHA-256 $hash"
