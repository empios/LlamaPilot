#!/bin/bash
set -euo pipefail
image_path="$1"
mount_path=$(mktemp -d /private/tmp/llamapilot-dmg.XXXXXX)
cleanup() {
  hdiutil detach "$mount_path" -quiet || true
  rmdir "$mount_path" 2>/dev/null || true
}
trap cleanup EXIT
hdiutil verify "$image_path" -quiet
hdiutil attach "$image_path" -readonly -nobrowse -mountpoint "$mount_path" -quiet
codesign --verify --deep --strict --verbose=2 "$mount_path/LlamaPilot.app"
file "$mount_path/LlamaPilot.app/Contents/MacOS/llamapilot"
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$mount_path/LlamaPilot.app/Contents/Info.plist"
