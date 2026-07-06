#!/usr/bin/env bash
# Remove jereide from the host platform.
#
# Usage: ./uninstall.sh [--system]
#   --system  Remove system-wide (Linux) install from /usr/local/bin
#             and /usr/share (requires sudo).
#   Default:  Remove user-local install from ~/.local (Linux) or the
#             macOS /Applications bundles + /usr/local/bin symlinks.
#
# Windows: Git Bash / MSYS2 runs this script; removes the extracted
# zip layout if called from it. For the official Inno Setup
# installer, use "Add or remove programs" in Windows Settings.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SYSTEM=0
for arg in "$@"; do
    case "$arg" in
        --system) SYSTEM=1 ;;
        -h|--help)
            sed -n '2,13p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "error: unknown argument: $arg" >&2; exit 1 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }
removed=0
rm_if_exists() {
    local path="$1"
    if [ -e "$path" ] || [ -L "$path" ]; then
        $sudo_cmd rm -rf "$path" && {
            echo "  removed: $path"
            removed=$((removed + 1))
        }
    fi
}

uninstall_linux() {
    local bin_dir share_prefix app_dir icon_dir
    if [ "$SYSTEM" -eq 1 ]; then
        bin_dir=/usr/local/bin
        share_prefix=/usr/local/share
        app_dir=/usr/share/applications
        icon_dir=/usr/share/icons/hicolor/256x256/apps
        sudo_cmd=sudo
    else
        bin_dir="$HOME/.local/bin"
        share_prefix="$HOME/.local/share"
        app_dir="$HOME/.local/share/applications"
        icon_dir="$HOME/.local/share/icons/hicolor/256x256/apps"
        sudo_cmd=
    fi

    # Binaries (including any CLI symlinks).
    rm_if_exists "$bin_dir/jereide"

    # Per-app data directories.
    rm_if_exists "$share_prefix/jereide"

    # Desktop files.
    rm_if_exists "$app_dir/jereide.desktop"
    # Also clean the long reverse-DNS names if they were ever written.
    rm_if_exists "$app_dir/com.jeremy.jereide.desktop"

    # Hicolor icons (reverse-DNS names, plus legacy dashed names).
    rm_if_exists "$icon_dir/com.jeremy.jereide.png"
    rm_if_exists "$icon_dir/jereide.png"

    # Refresh caches so the taskbar / menu forgets us.
    local icon_root="${icon_dir%/256x256/apps}"
    $sudo_cmd rm -f "$icon_root/icon-theme.cache" 2>/dev/null || true
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        ${sudo_cmd:-} gtk-update-icon-cache -f -t "$icon_root" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        ${sudo_cmd:-} update-desktop-database "$app_dir" 2>/dev/null || true
    fi
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        ${sudo_cmd:-} kbuildsycoca6 --noincremental 2>/dev/null || true
    elif command -v kbuildsycoca5 >/dev/null 2>&1; then
        ${sudo_cmd:-} kbuildsycoca5 --noincremental 2>/dev/null || true
    fi
    rm -f "$HOME/.cache/icon-cache.kcache" 2>/dev/null || true

    if [ "$SYSTEM" -eq 0 ]; then
        # User uninstall: if the user previously ran `./install.sh --system`,
        # the system copies would now shadow any future user install with
        # stale versions. Warn about them so the user knows to run
        # `./uninstall.sh --system` if they want a full clean.
        local sys_warn=0
        for p in /usr/local/bin/jereide \
                 /usr/share/applications/jereide.desktop \
                 /usr/share/icons/hicolor/256x256/apps/com.jeremy.jereide.png \
                 /usr/share/icons/hicolor/256x256/apps/jereide.png; do
            [ -e "$p" ] && sys_warn=1 && break
        done
        if [ "$sys_warn" -eq 1 ]; then
            echo
            echo "Note: system-wide files remain in /usr/local or /usr/share."
            echo "      Run './uninstall.sh --system' to remove those too."
        fi
    fi
}

uninstall_macos() {
    # macOS installs to /Applications as .app bundles and leaves
    # /usr/local/bin/<name> symlinks. No `--system` distinction on Mac;
    # the flag is accepted but has no effect.
    local sudo_cmd=sudo
    rm_if_exists /Applications/JereIDE.app
    rm_if_exists /usr/local/bin/jereide

    # Launch Services cache keeps stale entries; rebuild it so Spotlight /
    # Dock forget the removed bundles.
    if [ -x /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister ]; then
        /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister \
            -kill -r -domain local -domain system -domain user 2>/dev/null || true
    fi
}

uninstall_windows() {
    # Inno Setup installs register with Windows' uninstaller. The right
    # path is Settings -> Apps -> "JereIDE" -> Uninstall. For zip-only
    # extracts there is nothing to unregister -- just delete the folder.
    cat <<'EOF'
Windows detected (Git Bash / MSYS).

If installed via the .exe installer:
  Open Settings -> Apps -> installed apps, find "JereIDE", click Uninstall.

If extracted from a zip: delete the folder you extracted into.
The per-user data directory at %APPDATA%\jereide can be removed
manually; it holds your settings, sessions, and recent files.
EOF
}

OS="$(uname -s 2>/dev/null || echo Unknown)"
case "$OS" in
    Linux)
        uninstall_linux
        ;;
    Darwin)
        uninstall_macos
        ;;
    MINGW*|MSYS*|CYGWIN*)
        uninstall_windows
        ;;
    *)
        die "unsupported OS: $OS"
        ;;
esac

echo
if [ "$removed" -eq 0 ]; then
    echo "No files were removed (nothing to uninstall at this scope)."
else
    echo "Uninstalled $removed item(s)."
fi
