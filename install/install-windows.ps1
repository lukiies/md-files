# Install the md-files tool suite on Windows.
#
# Builds the release binaries and copies them to a per-user bin folder,
# adds that folder to the user PATH, registers mdread in the "Open With"
# list for .md/.markdown files, and adds Explorer right-click entries for
# all three tools (Open in mdread / Convert to HTML / Convert to Word).
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
$mdExts = @(".md", ".markdown")
$menuVerbs = @("mdfiles.read", "mdfiles.tohtml", "mdfiles.todocx")

if ($Uninstall) {
    foreach ($t in $tools) {
        $p = Join-Path $binDir $t
        if (Test-Path $p) { Remove-Item $p -Force }
    }
    if ((Test-Path $binDir) -and -not (Get-ChildItem $binDir)) { Remove-Item $binDir }
    # Remove Open With registration
    Remove-Item -Path "HKCU:\Software\Classes\Applications\mdread.exe" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "HKCU:\Software\Classes\mdfiles.mdread" -Recurse -Force -ErrorAction SilentlyContinue
    # Remove the Explorer context-menu entries
    foreach ($ext in $mdExts) {
        foreach ($verb in $menuVerbs) {
            Remove-Item -Path "HKCU:\Software\Classes\SystemFileAssociations\$ext\shell\$verb" -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
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

# 4. Register mdread in the Open With list for markdown files and claim the
#    extension default. The per-user default is HKCU\Software\Classes\<ext>'s
#    (Default) progid; Explorer uses it whenever no (hash-protected)
#    UserChoice overrides it - Windows may still ask once to confirm.
$exe = Join-Path $binDir "mdread.exe"
$progId = "mdfiles.mdread"
New-Item -Path "HKCU:\Software\Classes\$progId\shell\open\command" -Force | Out-Null
New-Item -Path "HKCU:\Software\Classes\$progId\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId" -Name "(Default)" -Value "Markdown Document"
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId\DefaultIcon" -Name "(Default)" -Value "`"$exe`",0"
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId\shell\open\command" -Name "(Default)" -Value "`"$exe`" `"%1`""
foreach ($ext in ".md", ".markdown") {
    New-Item -Path "HKCU:\Software\Classes\$ext\OpenWithProgids" -Force | Out-Null
    New-ItemProperty -Path "HKCU:\Software\Classes\$ext\OpenWithProgids" -Name $progId -Value ([byte[]]@()) -PropertyType None -Force | Out-Null
    Set-ItemProperty -Path "HKCU:\Software\Classes\$ext" -Name "(Default)" -Value $progId
}
Write-Host "mdread registered for .md/.markdown (Open With + extension default progid)."

# 5. Explorer right-click entries for all three tools on .md/.markdown.
#    SystemFileAssociations verbs work regardless of which app owns the
#    extension. Icon = the tool's exe. (On Windows 11 these appear in the
#    classic menu - "Show more options" / Shift+F10 - and in the compact
#    menu once a default app is chosen.)
$menu = @(
    @{ Verb = "mdfiles.read";   Label = "Open in mdread";           Exe = "mdread.exe";  Args = '"%1"' },
    @{ Verb = "mdfiles.tohtml"; Label = "Convert to HTML (md2html)"; Exe = "md2html.exe"; Args = '--save "%1"' },
    @{ Verb = "mdfiles.todocx"; Label = "Convert to Word (md2docx)"; Exe = "md2docx.exe"; Args = '"%1"' }
)
foreach ($ext in $mdExts) {
    foreach ($m in $menu) {
        $exePath = Join-Path $binDir $m.Exe
        $key = "HKCU:\Software\Classes\SystemFileAssociations\$ext\shell\$($m.Verb)"
        New-Item -Path "$key\command" -Force | Out-Null
        Set-ItemProperty -Path $key -Name "(Default)" -Value $m.Label
        Set-ItemProperty -Path $key -Name "Icon" -Value "`"$exePath`""
        Set-ItemProperty -Path "$key\command" -Name "(Default)" -Value "`"$exePath`" $($m.Args)"
    }
}
Write-Host "Explorer right-click entries added for .md/.markdown (Open in mdread, Convert to HTML, Convert to Word)."
Write-Host "To make mdread the default viewer: right-click a .md file > Open with > Choose another app > mdread."
Write-Host "Done."
