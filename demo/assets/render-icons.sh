#!/bin/bash
# Renders the demo's app icon, drawn in the SVGs here, to the PNGs the phones take; rerun it
# after changing them. The desktops take app-icon.svg as it is (src/lib.rs, build.rs).
#
# - iOS: app-icon-square.svg, full-bleed (iOS rounds the corners), 1024 px and opaque, into
#   the asset catalog in ios/.
# - Android: app-icon-foreground.svg as the adaptive icon's foreground (and monochrome), on
#   the background colour in values/; app-icon.svg as the icon of Android 7 (API 24, 25),
#   which has no adaptive icons.
#
# Needs rsvg-convert (librsvg) and ImageMagick's magick.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
demo="$(cd "$here/.." && pwd)"

ios="$demo/ios/Assets.xcassets/AppIcon.appiconset"
mkdir -p "$ios"
rsvg-convert -w 1024 -h 1024 "$here/app-icon-square.svg" \
  | magick - -background '#1d2433' -alpha remove -alpha off "$ios/AppIcon.png"

res="$demo/android/app/src/main/res"
# The densities' scales against mdpi: a 48 dp legacy icon, a 108 dp adaptive layer.
for density in mdpi:1 hdpi:1.5 xhdpi:2 xxhdpi:3 xxxhdpi:4; do
  name="${density%%:*}"
  scale="${density##*:}"
  dir="$res/mipmap-$name"
  mkdir -p "$dir"
  legacy=$(awk "BEGIN { print 48 * $scale }")
  layer=$(awk "BEGIN { print 108 * $scale }")
  rsvg-convert -w "$legacy" -h "$legacy" "$here/app-icon.svg" -o "$dir/ic_launcher.png"
  rsvg-convert -w "$layer" -h "$layer" "$here/app-icon-foreground.svg" \
    -o "$dir/ic_launcher_foreground.png"
done
