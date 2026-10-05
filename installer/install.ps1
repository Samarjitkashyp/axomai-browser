# Axomai Browser installer (run by Axomai-Setup-<version>.exe, an IExpress self-extracting package).
# Installs for the current user only (no administrator rights): files, Start menu and desktop shortcuts, and an entry
# under "Installed apps" so it can be removed. Your bookmarks, history and settings live in %APPDATA%\AxomaiBrowser
# and are never touched by installing or updating.
param(
    [string]$Dir = "",
    [switch]$Quiet,
    [switch]$NoShortcuts,
    [switch]$Launch
)
$ErrorActionPreference = 'Stop'

if (-not $Dir) {
    if ($env:AXOMAI_INSTALL_DIR) { $Dir = $env:AXOMAI_INSTALL_DIR }
    else { $Dir = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Axomai Browser' }
}

function Say($msg) {
    if ($Quiet) { return }
    Add-Type -AssemblyName System.Windows.Forms
    [void][System.Windows.Forms.MessageBox]::Show($msg, 'Axomai Browser Setup')
}

try {
    # Close a running copy that lives in the install folder (an update replaces its files).
    Get-Process axomai_browser -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($Dir, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object {
        [void]$_.CloseMainWindow()
        Start-Sleep -Seconds 2
        if (-not $_.HasExited) { $_ | Stop-Process -Force }
    }
    Start-Sleep -Milliseconds 500

    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    Expand-Archive -LiteralPath (Join-Path $PSScriptRoot 'payload.zip') -DestinationPath $Dir -Force

    $exe = Join-Path $Dir 'axomai_browser.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'The program file is missing from the installer.' }
    $version = '0'
    $vf = Join-Path $Dir 'version.txt'
    if (Test-Path -LiteralPath $vf) { $version = (Get-Content -LiteralPath $vf -Raw).Trim() }

    if (-not $NoShortcuts) {
        $ws = New-Object -ComObject WScript.Shell
        foreach ($folder in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('DesktopDirectory'))) {
            $lnk = $ws.CreateShortcut((Join-Path $folder 'Axomai Browser.lnk'))
            $lnk.TargetPath = $exe
            $lnk.WorkingDirectory = $Dir
            $lnk.IconLocation = $exe
            $lnk.Description = 'Axomai Browser'
            $lnk.Save()
        }
    }

    $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AxomaiBrowser'
    New-Item -Path $key -Force | Out-Null
    $size = [int]((Get-ChildItem -LiteralPath $Dir -Recurse -File | Measure-Object Length -Sum).Sum / 1KB)
    $props = @{
        DisplayName     = 'Axomai Browser'
        DisplayVersion  = $version
        Publisher       = 'Samarjit Kashyap'
        InstallLocation = $Dir
        DisplayIcon     = $exe
        UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $Dir 'uninstall.ps1')`""
        EstimatedSize   = $size
    }
    foreach ($k in $props.Keys) { Set-ItemProperty -Path $key -Name $k -Value $props[$k] }
    Set-ItemProperty -Path $key -Name NoModify -Value 1 -Type DWord
    Set-ItemProperty -Path $key -Name NoRepair -Value 1 -Type DWord

    if ($Launch) { Start-Process -FilePath $exe -WorkingDirectory $Dir }
    Say "Axomai Browser $version is installed.`n`nFind it in the Start menu or on the desktop."
    exit 0
}
catch {
    Say ("Axomai Browser could not be installed:`n`n" + $_.Exception.Message)
    exit 1
}
