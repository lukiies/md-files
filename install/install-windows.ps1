# Install the md-files tool suite on Windows.
#
# Builds the release binaries and copies them to a per-user bin folder,
# adds that folder to the user PATH, and registers mdread in the
# "Open With" list for .md/.markdown files.
#
# Run from the repository root:
#   powershell -ExecutionPolicy Bypass -File install\install-windows.ps1
#
# Uninstall:
#   powershell -ExecutionPolicy Bypass -File install\install-windows.ps1 -Uninstall

param([switch]$Uninstall)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$binDir = Join-Path $env:LOCALAPPDATA "Programs\md-files"
$tools = @("mdread.exe", "md2html.exe", "md2docx.exe")

if ($Uninstall) {
    foreach ($t in $tools) {
        $p = Join-Path $binDir $t
        if (Test-Path $p) { Remove-Item $p -Force }
    }
    if ((Test-Path $binDir) -and -not (Get-ChildItem $binDir)) { Remove-Item $binDir }
    # Remove Open With registration
    Remove-Item -Path "HKCU:\Software\Classes\Applications\mdread.exe" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "HKCU:\Software\Classes\mdfiles.mdread" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "md-files uninstalled from $binDir (PATH entry left in place; remove manually if wanted)."
    exit 0
}

# 1. Build
Write-Host "Building release binaries..."
Push-Location $repo
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

# 2. Copy to per-user bin
New-Item -ItemType Directory -Force $binDir | Out-Null
foreach ($t in $tools) {
    Copy-Item (Join-Path $repo "target\release\$t") (Join-Path $binDir $t) -Force
}
Write-Host "Installed to $binDir"

# 3. Add to user PATH if missing
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$binDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$binDir", "User")
    Write-Host "Added $binDir to the user PATH (new terminals will pick it up)."
}

# 4. Register mdread in the Open With list for markdown files
$exe = Join-Path $binDir "mdread.exe"
$progId = "mdfiles.mdread"
New-Item -Path "HKCU:\Software\Classes\$progId\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId" -Name "(Default)" -Value "Markdown Document"
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId\shell\open\command" -Name "(Default)" -Value "`"$exe`" `"%1`""
foreach ($ext in ".md", ".markdown") {
    New-Item -Path "HKCU:\Software\Classes\$ext\OpenWithProgids" -Force | Out-Null
    New-ItemProperty -Path "HKCU:\Software\Classes\$ext\OpenWithProgids" -Name $progId -Value ([byte[]]@()) -PropertyType None -Force | Out-Null
}
Write-Host "mdread registered in the Open With list for .md/.markdown."
Write-Host "To make it the default viewer: right-click a .md file > Open with > Choose another app > mdread."
Write-Host "Done."
