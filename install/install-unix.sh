#!/bin/sh
# Install the md-files tool suite on Linux or macOS.
#
# Builds the release binaries and copies them to ~/.local/bin (Linux) or
# /usr/local/bin (macOS, may prompt for sudo). On Linux a .desktop entry is
# added so mdread appears in "Open With" for markdown files.
#
# Prerequisites:
#   - Rust toolchain (https://rustup.rs)
#   - Linux: WebKitGTK dev packages, e.g.
#       Debian/Ubuntu: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev
#       Fedora:        sudo dnf install webkit2gtk4.1-devel gtk3-devel
#   - macOS: nothing extra (uses the system WKWebView)
#
# Run from the repository root:
#   sh install/install-unix.sh
#
# Uninstall:
#   sh install/install-unix.sh uninstall

set -e
repo="$(cd "$(dirname "$0")/.." && pwd)"
tools="mdread md2html md2docx"

case "$(uname -s)" in
    Darwin) bindir="/usr/local/bin"; sudo_cmd="sudo" ;;
    *)      bindir="$HOME/.local/bin"; sudo_cmd="" ;;
esac

if [ "$1" = "uninstall" ]; then
    for t in $tools; do
        $sudo_cmd rm -f "$bindir/$t"
    done
    rm -f "$HOME/.local/share/applications/mdread.desktop" 2>/dev/null || true
    echo "md-files uninstalled from $bindir."
    exit 0
fi

echo "Building release binaries..."
cd "$repo"
cargo build --release

mkdir -p "$bindir"
for t in $tools; do
    $sudo_cmd install -m 755 "$repo/target/release/$t" "$bindir/$t"
done
echo "Installed to $bindir"

case ":$PATH:" in
    *":$bindir:"*) ;;
    *) echo "NOTE: $bindir is not on your PATH - add it in your shell profile." ;;
esac

# Linux: desktop entry so mdread shows up in Open With for markdown files
if [ "$(uname -s)" != "Darwin" ]; then
    appdir="$HOME/.local/share/applications"
    mkdir -p "$appdir"
    cat > "$appdir/mdread.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=mdread
Comment=Fast Markdown reader
Exec=$bindir/mdread %f
Terminal=false
Categories=Utility;Viewer;
MimeType=text/markdown;
EOF
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$appdir" || true
    echo "mdread desktop entry installed (Open With > mdread for .md files)."
fi

echo "Done."
