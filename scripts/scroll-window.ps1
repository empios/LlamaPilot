# Scrolls the app window at a given point. Development helper only; not part of the application.
param(
    [string]$ProcessName = "llama-control",
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y,
    # Negative scrolls down, positive scrolls up, in notches.
    [int]$Notches = -3
)

$signature = @'
using System;
using System.Runtime.InteropServices;

public static class WindowScroll {
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, uint dx, uint dy, int dwData, IntPtr dwExtraInfo);

    public const uint WHEEL = 0x0800;

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
}
'@

if (-not ("WindowScroll" -as [type])) {
    Add-Type -TypeDefinition $signature
}

$process = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1

if (-not $process) {
    Write-Error "No window found for process '$ProcessName'."
    exit 1
}

[void][WindowScroll]::SetForegroundWindow($process.MainWindowHandle)
Start-Sleep -Milliseconds 250

$rect = New-Object WindowScroll+RECT
[void][WindowScroll]::GetWindowRect($process.MainWindowHandle, [ref]$rect)
[void][WindowScroll]::SetCursorPos(($rect.Left + $X), ($rect.Top + $Y))
Start-Sleep -Milliseconds 120

[WindowScroll]::mouse_event([WindowScroll]::WHEEL, 0, 0, ($Notches * 120), [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

"Scrolled $Notches notches at window-relative ($X, $Y)"
