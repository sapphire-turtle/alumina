#!/bin/bash
# Builds the demo for a real iPhone or iPad, installs it and launches it, printing its log
# until Ctrl-C. Needs DEVELOPMENT_TEAM in Local.xcconfig, and the device paired with this
# Mac (by cable once; Wi-Fi after), unlocked and in Developer Mode.
#
#   ./run-device.sh                 # the first paired device
#   DEVICE=<name or UDID> ./run-device.sh
#   CONFIGURATION=Release ./run-device.sh   # how fast it scrolls without the debug build
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cd "$here"

if ! grep -qs '^DEVELOPMENT_TEAM *= *[A-Z0-9]' Local.xcconfig; then
  echo "error: set DEVELOPMENT_TEAM in ios/Local.xcconfig (cp Local.xcconfig.example Local.xcconfig)" >&2
  exit 1
fi

# The device: DEVICE, or the first physical one devicectl knows.
if [ -z "${DEVICE:-}" ]; then
  json="$(mktemp)"
  trap 'rm -f "$json"' EXIT
  xcrun devicectl list devices --quiet --json-output "$json" >/dev/null
  DEVICE="$(/usr/bin/python3 -c '
import json, sys
for d in json.load(open(sys.argv[1]))["result"]["devices"]:
    if d.get("hardwareProperties", {}).get("reality") == "physical" \
       and d.get("connectionProperties", {}).get("pairingState") == "paired":
        print(d["hardwareProperties"]["udid"]); break
' "$json")"
  if [ -z "$DEVICE" ]; then
    echo "error: no paired device; connect one by cable and trust this Mac" >&2
    exit 1
  fi
fi
echo "run-device: device=$DEVICE"

[ -d AluminaDemo.xcodeproj ] || xcodegen generate

configuration="${CONFIGURATION:-Debug}"
# -allowProvisioningUpdates lets Xcode register the device and make the profile itself.
xcodebuild -project AluminaDemo.xcodeproj -scheme AluminaDemo \
  -configuration "$configuration" -destination "generic/platform=iOS" \
  -derivedDataPath build -allowProvisioningUpdates build

app="build/Build/Products/$configuration-iphoneos/AluminaDemo.app"
bundle_id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Info.plist")"

xcrun devicectl device install app --device "$DEVICE" "$app"
xcrun devicectl device process launch --device "$DEVICE" --terminate-existing --console "$bundle_id"
