// Axomai Browser setup program. Built by build-installer.ps1 with the C# compiler that ships with Windows
// (no extra tools), with the application files embedded as payload.zip.
//
//   Axomai-Setup-<version>.exe            install with a confirmation message at the end
//   Axomai-Setup-<version>.exe /Q         quiet: no windows, starts the browser afterwards (used by auto-update)
//   Axomai-Setup-<version>.exe /DIR="C:\path"   install somewhere else
//
// It installs for the current user only (no administrator rights): files, Start menu and desktop shortcuts, and an
// entry under "Installed apps". Bookmarks, history and settings live in %APPDATA%\AxomaiBrowser and are never touched.
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Windows.Forms;
using Microsoft.Win32;

static class Setup
{
    static bool quiet;

    static void Say(string text)
    {
        if (!quiet) MessageBox.Show(text, "Axomai Browser Setup");
    }

    static string DefaultDir()
    {
        string env = Environment.GetEnvironmentVariable("AXOMAI_INSTALL_DIR");
        if (!string.IsNullOrEmpty(env)) return env;
        return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), @"Programs\Axomai Browser");
    }

    static void StopRunning(string dir)
    {
        foreach (Process p in Process.GetProcessesByName("axomai_browser"))
        {
            try
            {
                string path = p.MainModule.FileName;
                if (path == null || !path.StartsWith(dir, StringComparison.OrdinalIgnoreCase)) continue;
                p.CloseMainWindow();
                if (!p.WaitForExit(3000)) p.Kill();
                p.WaitForExit(3000);
            }
            catch (Exception) { /* a process we cannot inspect is not ours */ }
        }
    }

    static void Extract(string dir)
    {
        string root = Path.GetFullPath(dir).TrimEnd('\\') + "\\";
        using (Stream s = Assembly.GetExecutingAssembly().GetManifestResourceStream("payload.zip"))
        {
            if (s == null) throw new InvalidOperationException("This setup program has no files inside it.");
            using (ZipArchive zip = new ZipArchive(s, ZipArchiveMode.Read))
            {
                foreach (ZipArchiveEntry e in zip.Entries)
                {
                    string target = Path.GetFullPath(Path.Combine(dir, e.FullName.Replace('/', '\\')));
                    if (!target.StartsWith(root, StringComparison.OrdinalIgnoreCase)) throw new InvalidDataException("Unsafe path in the package: " + e.FullName);
                    if (e.FullName.EndsWith("/") || e.FullName.EndsWith("\\")) { Directory.CreateDirectory(target); continue; }
                    Directory.CreateDirectory(Path.GetDirectoryName(target));
                    e.ExtractToFile(target, true);
                }
            }
        }
    }

    static void Shortcut(string folder, string exe, string dir)
    {
        Type t = Type.GetTypeFromProgID("WScript.Shell");
        if (t == null) return;
        object shell = Activator.CreateInstance(t);
        object lnk = t.InvokeMember("CreateShortcut", BindingFlags.InvokeMethod, null, shell, new object[] { Path.Combine(folder, "Axomai Browser.lnk") });
        Type lt = lnk.GetType();
        lt.InvokeMember("TargetPath", BindingFlags.SetProperty, null, lnk, new object[] { exe });
        lt.InvokeMember("WorkingDirectory", BindingFlags.SetProperty, null, lnk, new object[] { dir });
        lt.InvokeMember("IconLocation", BindingFlags.SetProperty, null, lnk, new object[] { exe });
        lt.InvokeMember("Description", BindingFlags.SetProperty, null, lnk, new object[] { "Axomai Browser" });
        lt.InvokeMember("Save", BindingFlags.InvokeMethod, null, lnk, null);
    }

    static long FolderSizeKb(string dir)
    {
        long total = 0;
        foreach (string f in Directory.GetFiles(dir, "*", SearchOption.AllDirectories)) total += new FileInfo(f).Length;
        return total / 1024;
    }

    static int Main(string[] args)
    {
        string dir = null;
        bool noLaunch = false;
        foreach (string a in args)
        {
            string u = a.ToUpperInvariant();
            if (u == "/Q" || u == "/QUIET" || u == "/S") quiet = true;
            else if (u == "/NOLAUNCH") noLaunch = true;
            else if (u.StartsWith("/DIR=")) dir = a.Substring(5).Trim('"');
        }
        if (string.IsNullOrEmpty(dir)) dir = DefaultDir();
        try
        {
            StopRunning(dir);
            Directory.CreateDirectory(dir);
            Extract(dir);

            string exe = Path.Combine(dir, "axomai_browser.exe");
            if (!File.Exists(exe)) throw new FileNotFoundException("The program file is missing from the package.");
            string version = "0";
            string vf = Path.Combine(dir, "version.txt");
            if (File.Exists(vf)) version = File.ReadAllText(vf).Trim();

            if (Environment.GetEnvironmentVariable("AXOMAI_NO_SHORTCUTS") != "1")
            {
                Shortcut(Environment.GetFolderPath(Environment.SpecialFolder.Programs), exe, dir);
                Shortcut(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), exe, dir);
            }

            using (RegistryKey k = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\AxomaiBrowser"))
            {
                k.SetValue("DisplayName", "Axomai Browser");
                k.SetValue("DisplayVersion", version);
                k.SetValue("Publisher", "Samarjit Kashyap");
                k.SetValue("InstallLocation", dir);
                k.SetValue("DisplayIcon", exe);
                k.SetValue("UninstallString", "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"" + Path.Combine(dir, "uninstall.ps1") + "\"");
                k.SetValue("EstimatedSize", (int)FolderSizeKb(dir), RegistryValueKind.DWord);
                k.SetValue("NoModify", 1, RegistryValueKind.DWord);
                k.SetValue("NoRepair", 1, RegistryValueKind.DWord);
            }

            if (quiet && !noLaunch) Process.Start(new ProcessStartInfo(exe) { WorkingDirectory = dir, UseShellExecute = false });
            Say("Axomai Browser " + version + " is installed.\n\nFind it in the Start menu or on the desktop.");
            return 0;
        }
        catch (Exception ex)
        {
            Say("Axomai Browser could not be installed:\n\n" + ex.Message);
            return 1;
        }
    }
}
