#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
desktop_id="io.github.BunnySweety.LumaWay"
env_file="$HOME/.config/lumaway/lumaway.env"
profile_file="$HOME/.config/lumaway/profiles/default.env"
desktop_file="$HOME/.local/share/applications/$desktop_id.desktop"
metainfo_file="$HOME/.local/share/metainfo/$desktop_id.metainfo.xml"

cargo build --release -p lumaway-cli -p lumaway-gui

install -Dm755 "$repo_root/target/release/lumaway" "$HOME/.local/bin/lumaway"
install -Dm755 "$repo_root/target/release/lumaway-gui" "$HOME/.local/bin/lumaway-gui"

if [[ ! -e "$env_file" ]]; then
    install -Dm600 "$repo_root/packaging/desktop/lumaway.env.example" "$env_file"
    echo "Created $env_file. Complete the local configuration before starting LumaWay."
else
    chmod 600 "$env_file"
    echo "Kept existing $env_file."
fi

if [[ ! -e "$profile_file" ]]; then
    install -Dm644 "$repo_root/packaging/desktop/default.profile.env" "$profile_file"
    echo "Created $profile_file."
else
    echo "Kept existing $profile_file."
fi

install -d "$HOME/.local/share/applications"
sed "s|@HOME@|$HOME|g" "$repo_root/packaging/desktop/$desktop_id.desktop.in" > "$desktop_file"
chmod 644 "$desktop_file"

install -Dm644 "$repo_root/packaging/desktop/$desktop_id.metainfo.xml.in" "$metainfo_file"

if command -v msgfmt >/dev/null 2>&1 && [[ -f "$repo_root/po/LINGUAS" ]]; then
    while IFS= read -r lang; do
        [[ -z "$lang" || "$lang" == \#* ]] && continue
        po_file="$repo_root/po/$lang.po"
        mo_file="$HOME/.local/share/locale/$lang/LC_MESSAGES/lumaway-gui.mo"
        if [[ -f "$po_file" ]]; then
            install -d "$(dirname "$mo_file")"
            msgfmt --check -o "$mo_file" "$po_file"
        fi
    done < "$repo_root/po/LINGUAS"
else
    echo "msgfmt not found; skipped locale compilation."
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
fi

if command -v appstreamcli >/dev/null 2>&1; then
    appstreamcli validate --no-net "$metainfo_file" >/dev/null 2>&1 || true
fi

cat <<EOF
Installed LumaWay desktop application.

Before first start, edit:
  $env_file

Capture/color defaults live in:
  $profile_file

Desktop metadata installed:
  $desktop_file
  $metainfo_file

Then launch LumaWay from the application menu, or run:
  gio launch $desktop_file

The application opens as a GTK/libadwaita window.
EOF
