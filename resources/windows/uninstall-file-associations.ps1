# Removes JereIDE file associations from Windows.
# Run as Administrator: powershell -ExecutionPolicy Bypass -File uninstall-file-associations.ps1

$AppId = "JereIDE.Editor"

Remove-Item -Path "HKCU:\Software\Classes\$AppId" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\Applications\jereide.exe" -Recurse -Force -ErrorAction SilentlyContinue

$extensions = Get-ChildItem "HKCU:\Software\Classes" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match '^\.' } |
    ForEach-Object { $_.PSChildName }

foreach ($ext in $extensions) {
    $key = "HKCU:\Software\Classes\$ext\OpenWithProgids"
    if (Test-Path $key) {
        Remove-ItemProperty -Path $key -Name $AppId -ErrorAction SilentlyContinue
    }
}

Write-Host "JereIDE file associations removed."
