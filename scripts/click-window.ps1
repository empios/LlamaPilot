# Clicks a point inside the app window, addressed in window-relative pixels so coordinates can
# be read straight off a capture. Development helper only; not part of the application.
param(
    [string]$ProcessName = "llama-control",
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y
)

$signature = @'
using System;
using System.Runtime.InteropServices;

public static class WindowInput {
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, IntPtr dwExtraInfo);

    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
}
'@

if (-not ("WindowInput" -as [type])) {
    Add-Type -TypeDefinition $signature
}

$process = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1

if (-not $process) {
    Write-Error "No window found for process '$ProcessName'."
    exit 1
}

$handle = $process.MainWindowHandle
[void][WindowInput]::SetForegroundWindow($handle)
Start-Sleep -Milliseconds 300

$rect = New-Object WindowInput+RECT
[void][WindowInput]::GetWindowRect($handle, [ref]$rect)

$screenX = $rect.Left + $X
$screenY = $rect.Top + $Y

[void][WindowInput]::SetCursorPos($screenX, $screenY)
Start-Sleep -Milliseconds 120
[WindowInput]::mouse_event([WindowInput]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 60
[WindowInput]::mouse_event([WindowInput]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

"Clicked window-relative ($X, $Y) => screen ($screenX, $screenY)"
