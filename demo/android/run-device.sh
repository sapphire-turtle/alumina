#!/bin/bash
# Builds the demo for a real Android phone or tablet (arm64-v8a), installs it and launches
# it, printing its log until Ctrl-C. Needs what build-rust.sh and Gradle need, and the
# device in USB debugging (or paired for wireless debugging) and listed by `adb devices`.
#
#   ./run-device.sh                 # the only device adb sees
#   ANDROID_SERIAL=<serial> ./run-device.sh
#   ABIS=x86_64 ./run-device.sh     # an emulator on an x86_64 host
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
package=org.sapphire_turtle.alumina_demo

# adb takes the device from ANDROID_SERIAL; without it, fail early if there is none.
if [ -z "${ANDROID_SERIAL:-}" ] && ! adb get-state >/dev/null 2>&1; then
  echo "error: no device; check \`adb devices\` (USB debugging on, this computer allowed)" >&2
  exit 1
fi

ABIS="${ABIS:-arm64-v8a}" "$here/build-rust.sh"
gradle -p "$here" assembleDebug

adb install -r "$here/app/build/outputs/apk/debug/app-debug.apk"
adb logcat -c
adb shell am start -S -n "$package/.MainActivity"
adb logcat -s alumina-demo
