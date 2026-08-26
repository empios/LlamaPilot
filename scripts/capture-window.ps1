# Captures a screenshot of the running app window so UI changes can be reviewed without a
# human in the loop. Development helper only; not part of the application.
param(
    [string]$ProcessName = "llamapilot",
    [Parameter(Mandatory = $true)][string]$OutputPath
)

Add-Type -AssemblyName System.Drawing

$signature = @'
using System;
using System.Runtime.InteropServices;

public static class WindowCapture {
    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
}
'@

if (-not ("WindowCapture" -as [type])) {
    Add-Type -TypeDefinition $signature -ReferencedAssemblies System.Drawing
}

$process = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1

if (-not $process) {
    Write-Error "No window found for process '$ProcessName'. Is the app running?"
    exit 1
}

$handle = $process.MainWindowHandle

[void][WindowCapture]::SetForegroundWindow($handle)
Start-Sleep -Milliseconds 400

$rect = New-Object WindowCapture+RECT
[void][WindowCapture]::GetWindowRect($handle, [ref]$rect)
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

if ($width -le 0 -or $height -le 0) {
    Write-Error "Window has no drawable area ($width x $height)."
    exit 1
}

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)

# PW_RENDERFULLCONTENT (2) is required for WebView2 content to appear in the capture.
$deviceContext = $graphics.GetHdc()
$printed = [WindowCapture]::PrintWindow($handle, $deviceContext, 2)
$graphics.ReleaseHdc($deviceContext)

if (-not $printed) {
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
}

$bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

"Captured ${width}x${height} to $OutputPath"
