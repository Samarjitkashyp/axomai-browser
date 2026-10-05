# Removes Axomai Browser (installed by Axomai-Setup). Your bookmarks, history, passwords and settings stay in
# %APPDATA%\AxomaiBrowser unless you pass -RemoveData.
param(
    [switch]$Quiet,
    [switch]$RemoveData
)
$ErrorActionPreference = 'Continue'
$Dir = $PSScriptRoot

Get-Process axomai_browser -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($Dir, [StringComparison]::OrdinalIgnoreCase) } | ForEach-Object {
    [void]$_.CloseMainWindow()
    Start-Sleep -Seconds 2
    if (-not $_.HasExited) { $_ | Stop-Process -Force }
}

foreach ($folder in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('DesktopDirectory'))) {
    $lnk = Join-Path $folder 'Axomai Browser.lnk'
    if (Test-Path -LiteralPath $lnk) {
        # only remove a shortcut that points at this installation
        $target = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk).TargetPath
        if ($target -and $target.StartsWith($Dir, [StringComparison]::OrdinalIgnoreCase)) { Remove-Item -LiteralPath $lnk -Force }
    }
}
Remove-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AxomaiBrowser' -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveData) {
    $data = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'AxomaiBrowser'
    if (Test-Path -LiteralPath $data) { Remove-Item -LiteralPath $data -Recurse -Force -ErrorAction SilentlyContinue }
}

# This script lives in the folder being removed, so a helper deletes the folder once the script has finished.
Start-Process -WindowStyle Hidden -FilePath 'cmd.exe' -ArgumentList "/c ping -n 3 127.0.0.1 >nul & rmdir /s /q `"$Dir`""
if (-not $Quiet) {
    Add-Type -AssemblyName System.Windows.Forms
    [void][System.Windows.Forms.MessageBox]::Show("Axomai Browser was removed. Your bookmarks, history and settings were kept in %APPDATA%\AxomaiBrowser.", 'Axomai Browser')
}
