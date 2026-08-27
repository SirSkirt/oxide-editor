#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <rivet.deb>" >&2
  exit 2
fi

deb="$1"
if [[ ! -f "$deb" ]]; then
  echo "Debian package not found: $deb" >&2
  exit 3
fi

package_name="$(dpkg-deb -f "$deb" Package)"

# B1.3.6 renamed the visible product from Oxide Editor to Rivet. Tauri derives
# the Debian package name from productName, so the first Rivet .deb became a
# different package ('rivet') while still installing files previously owned by
# 'oxide-editor'. Debian correctly refused that file takeover.
#
# Keep Rivet as the new package identity, but explicitly declare the one-time
# package migration so dpkg can replace an existing Oxide installation.
if [[ "$package_name" == "oxide-editor" ]]; then
  echo "Debian package already uses the legacy package identity; no migration metadata needed."
  exit 0
fi

if [[ "$package_name" != "rivet" ]]; then
  echo "Unexpected Debian package identity '$package_name'; refusing to patch migration metadata." >&2
  exit 4
fi

workdir="$(mktemp -d)"
cleanup() { rm -rf "$workdir"; }
trap cleanup EXIT

dpkg-deb -R "$deb" "$workdir/root"
control="$workdir/root/DEBIAN/control"

python3 - "$control" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(encoding='utf-8')
fields = {
    'Provides': 'oxide-editor',
    'Replaces': 'oxide-editor',
    'Conflicts': 'oxide-editor',
}

lines = text.splitlines()
seen = set()
out = []
for line in lines:
    if ':' in line:
        key = line.split(':', 1)[0]
        if key in fields:
            out.append(f'{key}: {fields[key]}')
            seen.add(key)
            continue
    out.append(line)
for key, value in fields.items():
    if key not in seen:
        out.append(f'{key}: {value}')
path.write_text('\n'.join(out).rstrip() + '\n', encoding='utf-8')
PY

patched="$workdir/rivet-patched.deb"
dpkg-deb --root-owner-group -b "$workdir/root" "$patched" >/dev/null
mv "$patched" "$deb"

echo "Patched Debian migration metadata:"
dpkg-deb -f "$deb" Package Version Provides Replaces Conflicts
