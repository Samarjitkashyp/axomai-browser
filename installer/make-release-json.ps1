# Writes release.json next to a built installer, in the format the browser's update check reads.
#   powershell -File installer\make-release-json.ps1 -Dir <folder with Axomai-Setup-<version>.exe> -Version 4.1.0 -Notes "What's new"
# Upload the three files (exe, .sha256, release.json) to /var/www/axomai-browser/downloads/ on the server.
param(
    [Parameter(Mandatory = $true)][string]$Dir,
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Notes = "",
    [string]$Base = "https://axomai-browser.aiaxom.co.in/downloads"
)
$ErrorActionPreference = 'Stop'
$exe = "Axomai-Setup-$Version.exe"
foreach ($f in @($exe, "$exe.sha256")) {
    if (-not (Test-Path (Join-Path $Dir $f))) { throw "$f is not in $Dir (run build-installer.ps1 first)" }
}
$release = [ordered]@{
    tag_name = "v$Version"
    body     = $Notes
    assets   = @(
        [ordered]@{ name = $exe; browser_download_url = "$Base/$exe" },
        [ordered]@{ name = "$exe.sha256"; browser_download_url = "$Base/$exe.sha256" }
    )
}
$json = $release | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $Dir 'release.json'), $json, (New-Object System.Text.UTF8Encoding($false)))
Write-Output "Wrote $(Join-Path $Dir 'release.json')"
