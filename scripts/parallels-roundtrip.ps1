param(
    [string]$MutatedPath = "\\Mac\Home\Projects\loxone-portable\target\e2e-roundtrip\mutated.Loxone",
    [string]$ConfigSavedPath = "\\Mac\Home\Projects\loxone-portable\target\e2e-roundtrip\config_saved.Loxone",
    [string]$StatusOutPath = "\\Mac\Home\Projects\loxone-portable\target\e2e-roundtrip\vm_status.json",
    [string]$ScreenshotOutPath = "\\Mac\Home\Projects\loxone-portable\target\e2e-roundtrip\config_roundtrip.png"
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public class Win32Helper {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr hWndParent, EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder strText, int maxCount);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder strText, int maxCount);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    public static string GetTitle(IntPtr hWnd) {
        StringBuilder sb = new StringBuilder(512);
        GetWindowText(hWnd, sb, 512);
        return sb.ToString();
    }

    public static string GetClass(IntPtr hWnd) {
        StringBuilder sb = new StringBuilder(256);
        GetClassName(hWnd, sb, 256);
        return sb.ToString();
    }

    public static List<string> DescribeProcessWindows(uint pid) {
        var list = new List<string>();
        EnumWindows((hWnd, lParam) => {
            uint wPid;
            GetWindowThreadProcessId(hWnd, out wPid);
            if (wPid == pid && IsWindowVisible(hWnd)) {
                string t = GetTitle(hWnd);
                string c = GetClass(hWnd);
                var children = new List<string>();
                EnumChildWindows(hWnd, (ch, lp) => {
                    string ct = GetTitle(ch);
                    string cc = GetClass(ch);
                    if (!string.IsNullOrWhiteSpace(ct)) {
                        children.Add(cc + ":" + ct.Replace("\r", " ").Replace("\n", " "));
                    }
                    return true;
                }, IntPtr.Zero);
                list.Add(c + " [" + t + "] -> (" + string.Join(" | ", children) + ")");
            }
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
"@

$status = @{
    started_at = (Get-Date).ToString("o")
    mutated_path = $MutatedPath
    config_saved_path = $ConfigSavedPath
    opened = $false
    saved = $false
    main_window_title = ""
    dialogs_seen = @()
    windows_snapshot = @()
    file_size_before = 0
    file_size_after = 0
    last_write_before = ""
    last_write_after = ""
    error = $null
}

try {
    Get-Process -Name "LoxoneConfig" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2

    # Remove any Loxone auto-backup files in LocalAppData / Documents or target dir so the recovery popup doesn't block
    $localApp = $env:LOCALAPPDATA
    if ($localApp) {
        Get-ChildItem -Path "$localApp\Loxone" -Recurse -Filter "*config_saved*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
    }
    $docs = [Environment]::GetFolderPath("MyDocuments")
    if ($docs) {
        Get-ChildItem -Path "$docs\Loxone" -Recurse -Filter "*config_saved*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
    }

    $localWorkPath = "C:\Users\bartlomiej.klimczak\AppData\Local\Temp\e2e_roundtrip_work.Loxone"
    Copy-Item -Path $MutatedPath -Destination $localWorkPath -Force
    Copy-Item -Path $MutatedPath -Destination $ConfigSavedPath -Force
    $beforeInfo = Get-Item $localWorkPath
    $status.file_size_before = $beforeInfo.Length
    $status.last_write_before = $beforeInfo.LastWriteTimeUtc.ToString("o")

    $loxoneExe = "C:\Program Files (x86)\Loxone\LoxoneConfig\LoxoneConfig.exe"
    $proc = Start-Process -FilePath $loxoneExe -ArgumentList "`"$localWorkPath`"" -PassThru

    $deadline = (Get-Date).AddSeconds(35)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 900
        $p = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
        if ($null -eq $p) { break }

        $wins = [Win32Helper]::DescribeProcessWindows([uint32]$proc.Id)
        $status.windows_snapshot = $wins

        # If a modal dialog (#32770) is open, record it and dismiss it (send N / Esc / Enter)
        foreach ($w in $wins) {
            if ($w -like "#32770*") {
                if ($status.dialogs_seen -notcontains $w) {
                    $status.dialogs_seen += $w
                }
                [System.Windows.Forms.SendKeys]::SendWait("{TAB}{ENTER}")
                Start-Sleep -Milliseconds 500
            }
        }

        if ($p.MainWindowHandle -ne 0) {
            $title = $p.MainWindowTitle
            $status.main_window_title = $title
            if ($title -match "e2e_roundtrip_work") {
                $status.opened = $true
                break
            }
        }
    }

    Start-Sleep -Seconds 3
    $p = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if ($null -ne $p -and $p.MainWindowHandle -ne 0) {
        $status.main_window_title = $p.MainWindowTitle
        $status.windows_snapshot = [Win32Helper]::DescribeProcessWindows([uint32]$proc.Id)
        [Win32Helper]::ShowWindow($p.MainWindowHandle, 3) | Out-Null # SW_MAXIMIZE
        [Win32Helper]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
        Start-Sleep -Seconds 2

        # Modify project slightly or save directly: Ctrl+S saves if dirty, or click Save button, or nudge selection and Ctrl+S
        # Note: Loxone Config always writes the file on Ctrl+S even if clean, or we can check if Ctrl+S writes it
        [System.Windows.Forms.SendKeys]::SendWait("^s")
        Start-Sleep -Seconds 3

        # Capture full screenshot
        $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
        $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
        $graphics = [System.Drawing.Graphics]::FromImage($bmp)
        $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
        $bmp.Save($ScreenshotOutPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $graphics.Dispose()
        $bmp.Dispose()

        $afterInfo = Get-Item $localWorkPath
        $status.file_size_after = $afterInfo.Length
        $status.last_write_after = $afterInfo.LastWriteTimeUtc.ToString("o")
        $status.saved = ($status.last_write_after -ne $status.last_write_before) -or ($status.file_size_after -ne $status.file_size_before)
        if ($status.saved) {
            Copy-Item -Path $localWorkPath -Destination $ConfigSavedPath -Force
        }
    }
} catch {
    $status.error = $_.Exception.Message
}

$status.completed_at = (Get-Date).ToString("o")
$status | ConvertTo-Json -Depth 5 | Set-Content -Path $StatusOutPath -Encoding UTF8
