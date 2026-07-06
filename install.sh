#!/usr/bin/env bash
# Build and install jereide for the host platform.
# Delegates building to scripts/build-local-{linux,mac}.sh.
#
# Usage: ./install.sh [--system]
#   --system  Install system-wide to /usr/local (Linux only; requires sudo)
#   Default:  Install to ~/.local (Linux) or /Applications (macOS)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

SYSTEM=0
for arg in "$@"; do
    case "$arg" in
        --system) SYSTEM=1 ;;
        *) echo "error: unknown argument: $arg" >&2; exit 1 ;;
    esac
done

die() { echo "error: $*" >&2; exit 1; }

app_version() {
    awk -F'"' '
        /^\[workspace\.package\]$/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && $1 ~ /^version = / { print $2; exit }
    ' "$SCRIPT_DIR/Cargo.toml"
}

install_linux() {
    bash "$SCRIPT_DIR/scripts/build-local-linux.sh"

    local version stage_dir binary data_src
    version="$(app_version)"
    [ -n "$version" ] || die "could not determine version from Cargo.toml"
    stage_dir="$SCRIPT_DIR/dist/jereide-${version}-linux-x86_64"
    binary="$stage_dir/jereide"
    data_src="$stage_dir/data"

    [ -f "$binary" ] || die "binary not found at $binary"
    [ -d "$data_src" ] || die "data directory not found at $data_src"

    local bin_dir share_dir app_dir icon_dir sudo_cmd
    if [ "$SYSTEM" -eq 1 ]; then
        bin_dir=/usr/local/bin
        share_dir=/usr/local/share/jereide
        app_dir=/usr/share/applications
        icon_dir=/usr/share/icons/hicolor/256x256/apps
        sudo_cmd=sudo
    else
        bin_dir="$HOME/.local/bin"
        share_dir="$HOME/.local/share/jereide"
        app_dir="$HOME/.local/share/applications"
        icon_dir="$HOME/.local/share/icons/hicolor/256x256/apps"
        sudo_cmd=
    fi

    $sudo_cmd mkdir -p "$bin_dir" "$share_dir" "$app_dir" "$icon_dir"

    $sudo_cmd cp "$binary" "$bin_dir/jereide"
    $sudo_cmd chmod 755 "$bin_dir/jereide"

    # Sync data directory; remove stale files from a previous install.
    $sudo_cmd rsync -a --delete "$data_src/" "$share_dir/" 2>/dev/null \
        || { $sudo_cmd rm -rf "$share_dir"; $sudo_cmd cp -r "$data_src/." "$share_dir/"; }

    # SDL3 is statically linked — no libSDL3 to install next to the binary.

    $sudo_cmd cp "$stage_dir/com.jeremy.jereide.desktop" "$app_dir/jereide.desktop"
    # Theme icons are named by the dashless reverse-DNS app ID.
    $sudo_cmd cp "$stage_dir/com.jeremy.jereide.png" "$icon_dir/com.jeremy.jereide.png"
    # Drop legacy dashed icons left by older installs; the .desktop files
    # no longer reference them.
    $sudo_cmd rm -f "$icon_dir/lite-anvil.png" 2>/dev/null || true
    # Force a fresh mtime so any desktop env that watches dirs notices.
    $sudo_cmd touch "$icon_dir/com.jeremy.jereide.png" 2>/dev/null || true

    # User install: if ANY previous install ever put an anvil icon
    # system-wide (and left behind a stale / incomplete set), KDE's
    # KIconLoader prefers the system path and will fall back to the
    # mime-type icon for any app whose PNG is only in ~/.local. So
    # instead of wiping the system copies (which requires sudo), we
    # top them up: if a writable system hicolor dir already holds an
    # anvil icon, refresh the full reverse-DNS set alongside it so the
    # theme lookup resolves for every app. Silently no-op if the dirs
    # aren't writable.
    if [ "$SYSTEM" -eq 0 ]; then
        for sys_icons in \
            /usr/local/share/icons/hicolor/256x256/apps \
            /usr/share/icons/hicolor/256x256/apps; do
            if [ -w "$sys_icons" ]; then
                if [ -f "$sys_icons/com.jeremy.jereide.png" ] \
                   || [ -f "$sys_icons/lite-anvil.png" ]; then
                    cp -f "$stage_dir/com.jeremy.jereide.png" "$sys_icons/com.jeremy.jereide.png" 2>/dev/null || true
                    rm -f "$sys_icons/lite-anvil.png" 2>/dev/null || true
                    touch "$sys_icons/com.jeremy.jereide.png" 2>/dev/null || true
                    local sys_root="${sys_icons%/256x256/apps}"
                    rm -f "$sys_root/icon-theme.cache" 2>/dev/null || true
                    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
                        gtk-update-icon-cache -f -t "$sys_root" 2>/dev/null || true
                    fi
                fi
            fi
        done
    fi

    # System install: strip any pre-existing user-local .desktop / icon
    # copies so KDE's KIconLoader can't pick a stale user-local entry
    # with the wrong `Icon=` over the fresh system copy. XDG priority
    # has user-local entries shadowing system, which is why a prior
    # `./install.sh` run + now `./install.sh --system` can leave the
    # user-local shadow winning and the system icon ignored.
    if [ "$SYSTEM" -eq 1 ]; then
        rm -f "$HOME/.local/share/applications/jereide.desktop" \
              "$HOME/.local/share/icons/hicolor/256x256/apps/com.jeremy.jereide.png" \
              "$HOME/.local/bin/jereide" 2>/dev/null || true
        rm -f "$HOME/.local/share/icons/hicolor/icon-theme.cache" 2>/dev/null || true
        if command -v gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
        fi
    fi

    if command -v update-desktop-database >/dev/null 2>&1; then
        ${sudo_cmd:-} update-desktop-database "$app_dir" 2>/dev/null || true
    fi

    # Refresh the icon cache. `gtk-update-icon-cache -t` quietly bails
    # without an `index.theme`, which most user-installed hicolor roots
    # lack — so blow away any stale cache file first as a fallback so
    # GTK falls back to per-file scanning and picks up our new PNG.
    local icon_root="${icon_dir%/256x256/apps}"
    $sudo_cmd rm -f "$icon_root/icon-theme.cache" 2>/dev/null || true
    # Ensure the hicolor root has an `index.theme`. KDE/Plasma's
    # KIconLoader only treats a hicolor root as a real theme when it is
    # registered, so a user-install (`~/.local/share/icons/hicolor/`)
    # needs the root described for its icons to be discoverable. Writing
    # the minimum spec-compliant `index.theme` makes the lookup resolve
    # every hicolor-only icon this install ships.
    if [ ! -f "$icon_root/index.theme" ]; then
        $sudo_cmd tee "$icon_root/index.theme" >/dev/null <<'EOF'
[Icon Theme]
Name=Hicolor
Comment=Fallback icon theme
Directories=16x16/apps,22x22/apps,24x24/apps,32x32/apps,48x48/apps,64x64/apps,128x128/apps,256x256/apps,512x512/apps,scalable/apps

[16x16/apps]
Size=16
Type=Fixed
Context=Applications

[22x22/apps]
Size=22
Type=Fixed
Context=Applications

[24x24/apps]
Size=24
Type=Fixed
Context=Applications

[32x32/apps]
Size=32
Type=Fixed
Context=Applications

[48x48/apps]
Size=48
Type=Fixed
Context=Applications

[64x64/apps]
Size=64
Type=Fixed
Context=Applications

[128x128/apps]
Size=128
Type=Fixed
Context=Applications

[256x256/apps]
Size=256
Type=Fixed
Context=Applications

[512x512/apps]
Size=512
Type=Fixed
Context=Applications

[scalable/apps]
Size=48
Type=Scalable
MinSize=8
MaxSize=512
Context=Applications
EOF
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        ${sudo_cmd:-} gtk-update-icon-cache -f -t "$icon_root" 2>/dev/null || true
    fi
    # KDE / Plasma: rebuild the sycoca cache + the per-user icon cache
    # so newly-installed .desktop files and icons show up in the
    # taskbar without a session restart.
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        ${sudo_cmd:-} kbuildsycoca6 --noincremental 2>/dev/null || true
    elif command -v kbuildsycoca5 >/dev/null 2>&1; then
        ${sudo_cmd:-} kbuildsycoca5 --noincremental 2>/dev/null || true
    fi
    rm -f "$HOME/.cache/icon-cache.kcache" 2>/dev/null || true

    echo "Installed jereide to $bin_dir/"

    if [ "$SYSTEM" -eq 0 ] && [[ ":${PATH}:" != *":$HOME/.local/bin:"* ]]; then
        echo "Note: $HOME/.local/bin is not in PATH — add it to your shell profile."
    fi
}

install_macos() {
    bash "$SCRIPT_DIR/scripts/build-local-mac.sh"

    local built_app="$SCRIPT_DIR/dist/JereIDE.app"
    [ -d "$built_app" ] || die ".app bundle not found at $built_app"

    local app=/Applications/JereIDE.app
    rm -rf "$app"
    cp -R "$built_app" "$app"

    # Re-stamp ad-hoc signature after the copy so the install location matches the signed bundle.
    xattr -cr "$app" 2>/dev/null || true
    codesign --force --deep --sign - --timestamp=none "$app" >/dev/null 2>&1 || true

    local cli_link=/usr/local/bin/jereide
    if [ -L "$cli_link" ] || [ -f "$cli_link" ]; then
        sudo rm -f "$cli_link"
    fi
    sudo mkdir -p /usr/local/bin
    sudo ln -sf "$app/Contents/MacOS/jereide" "$cli_link"

    local version
    version="$(app_version)"
    echo "Installed JereIDE ${version:-?} to $app"
    echo "CLI symlink: $cli_link"

    # On stock macOS `/usr/local/bin` is wired into the default PATH via
    # `/etc/paths`, but Apple Silicon setups where the user has rewritten
    # PATH (e.g. to prefer Homebrew under `/opt/homebrew/bin`) often drop
    # it. Detect that and point the user at the fix rather than silently
    # leaving `jereide` un-runnable from the shell.
    if [[ ":${PATH}:" != *":/usr/local/bin:"* ]]; then
        local shell_rc
        case "${SHELL##*/}" in
            zsh)  shell_rc="$HOME/.zshrc" ;;
            bash) shell_rc="$HOME/.bash_profile" ;;
            fish) shell_rc="$HOME/.config/fish/config.fish" ;;
            *)    shell_rc="your shell profile" ;;
        esac
        echo
        echo "Note: /usr/local/bin is not in your PATH, so 'jereide'"
        echo "won't be runnable directly. Add it to"
        echo "$shell_rc — for zsh or bash:"
        echo
        echo "    export PATH=\"/usr/local/bin:\$PATH\""
        echo
    fi
}

OS="$(uname)"
case "$OS" in
    Linux)  install_linux ;;
    Darwin) install_macos ;;
    *)      die "unsupported OS: $OS" ;;
esac
