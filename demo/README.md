# alumina-demo

An app to try alumina's widgets by hand, on the desktops, iOS and Android. It is a
workspace member, built and tested with the rest, and never published.

| Page | What it shows |
|---|---|
| Date & time | The picker buttons (date, period, time, timestamp), a calendar on its own with what it picked, a drum and the time drums, and the calendar beside the drums. Settings: Japanese words, the first day of the week, week numbers, the dots, the cell size, the time's format, and which kinds the calendar picks |
| Markdown | The editor (`MarkdownEditor`) on a sample, laid out as Markdown as you type, with its toolbar (`MarkdownToolbar`), a Read-only checkbox and the last link clicked |

Below 600 points wide (a phone) the settings fold away above the widgets and the parts
stack.

The window is corundum-framework-gui's, as in the apps: on a phone it keeps the app clear of
the system's bars and the keyboard and feeds it the keyboard's text, on iOS it puts the
window in a scene, and everywhere it finds a CJK font for the Japanese words. The widget
crates themselves depend on neither corundum nor a window.

## Desktops (Linux, macOS, Windows)

```sh
cargo run -p alumina-demo
```

## iOS simulator

Needs an Apple-silicon Mac with Xcode, [XcodeGen](https://github.com/yonaskolb/XcodeGen)
and the iOS targets:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cd demo/ios
xcodegen generate
open AluminaDemo.xcodeproj   # pick a simulator and Run
```

Or from the command line:

```sh
xcodebuild -project AluminaDemo.xcodeproj -scheme AluminaDemo -sdk iphonesimulator \
  -configuration Debug -derivedDataPath build build
xcrun simctl list devices available            # pick one; names repeat across iOS versions
xcrun simctl boot <UDID>
open /Applications/Xcode.app/Contents/Applications/DeviceHub.app   # the screen
xcrun simctl install booted build/Build/Products/Debug-iphonesimulator/AluminaDemo.app
xcrun simctl launch --console-pty booted org.sapphire-turtle.alumina-demo
```

`simctl boot` starts the simulator without a window: open DeviceHub (Xcode 27; Simulator.app,
`open -a Simulator`, before it) to see it, or take a screenshot with
`xcrun simctl io booted screenshot shot.png`.

The project's pre-build script (`build-rust.sh`) builds the demo's staticlib for the SDK
Xcode targets; `Sources/main.m` hands the main thread to it.

## iOS device

A mouse on a simulator does not tell how a drag, a flick or a scroll feels under a finger;
try those on a phone (see [Trying it by hand](#trying-it-by-hand)). Needs a paid or free
Apple developer account signed in to Xcode (Settings → Accounts), and the device paired
with the Mac (connect it by cable once, trust the Mac; Wi-Fi works after) with Developer
Mode on (Settings → Privacy & Security, shown once Xcode has seen it).

Set your team, once:

```sh
cd demo/ios
cp Local.xcconfig.example Local.xcconfig
# DEVELOPMENT_TEAM = your team ID (Xcode → Settings → Accounts, or developer.apple.com)
# and PRODUCT_BUNDLE_IDENTIFIER = an id of your own, if the demo's is taken by another team
```

Then build, install and launch it, its log following until Ctrl-C:

```sh
./run-device.sh                          # the first paired device
DEVICE=<name or UDID> ./run-device.sh    # one of several (xcrun devicectl list devices)
CONFIGURATION=Release ./run-device.sh    # how smooth it is without the debug build
```

The first launch can fail as "untrusted developer" with a free account: trust it in
Settings → General → VPN & Device Management and run it again. In Xcode instead: open
`AluminaDemo.xcodeproj`, pick the device and Run (it signs with the team in
`Local.xcconfig`).

## Android emulator

Needs the Android SDK with an NDK (`ANDROID_NDK_HOME`),
[cargo-ndk](https://github.com/bbqsrc/cargo-ndk), Gradle 8.11 or later and JDK 17:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk
demo/android/build-rust.sh     # arm64-v8a and x86_64; ABIS=x86_64 for one
gradle -p demo/android assembleDebug
adb install -r demo/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.sapphire_turtle.alumina_demo/.MainActivity
adb logcat -s alumina-demo
```

`build-rust.sh` builds the cdylib and puts it in `app/src/main/jniLibs`; the app is
GameActivity (`MainActivity`), loading it and calling its `android_main`. Open
`demo/android` in Android Studio to run it from there instead.

A debug build runs because the workspace's `Cargo.toml` builds android-activity without
debug assertions (`[profile.dev.package.android-activity]`): android-activity 0.6 makes a
slice of the keyboard's text from a null pointer before there is any, and the check aborts.

## Android device

As for the emulator, with the phone in USB debugging (Settings → About phone → tap the
build number seven times; then Developer options → USB debugging) and this computer
allowed, so that `adb devices` lists it. Wireless debugging works too
(`adb pair`, then `adb connect`).

```sh
demo/android/run-device.sh                     # the only device adb sees
ANDROID_SERIAL=<serial> demo/android/run-device.sh
```

It builds the arm64-v8a library only (`ABIS` overrides), the APK, installs and starts it,
and follows its log until Ctrl-C.

## Trying it by hand

What a simulator's mouse cannot tell; on a device, with a finger:

- **The page's scroll**: drag the page up and down, flick it; it should follow the finger
  and keep going after a flick, and stop when touched.
- **The drums**: drag one, flick one (it keeps rolling and settles on a row), tap a row
  away from the middle (it rolls there). A drag that starts on a drum rolls the drum, not
  the page; one that starts beside it scrolls the page.
- **The calendar**: tap days, a range's two ends, the arrows, the heading (zooms out to
  the months and the years) and a month or a year (zooms back in). The cells should be big
  enough for a finger at the default size.
- **The picker buttons**: each opens its popup; a tap outside closes it. The text in the
  button takes the keyboard's input, and the page stays clear of the keyboard.
- **Turning the phone** (iPhone landscape, iPad every way): the page keeps clear of the
  notch or the island, the home indicator and the system bars.
- **iPad with a trackpad or a mouse**: two-finger scroll over the page and over a drum,
  where the drum takes it.
- **Dark and light**: the switch at the top right, and the system's setting with the third
  button.

## The icon

alumina's mark in white on a dark tile, drawn in [`assets/`](assets):

- `app-icon.svg`: the desktops' (the window's, and the Windows executable's through
  `build.rs`) and Android 7's;
- `app-icon-square.svg`: iOS's, full-bleed, as iOS rounds the corners itself;
- `app-icon-foreground.svg`: the foreground of Android's adaptive icon, from Android 8 on.

The phones take PNGs: after changing an SVG, run `assets/render-icons.sh` (needs
`rsvg-convert` and ImageMagick) and commit what it renders into `ios/Assets.xcassets` and
`android/app/src/main/res`.

## CI

CI builds and tests the demo on the desktops. Mobile CI runs clippy on it for iOS and
Android, builds the iOS app and the APK, starts each on the simulator and the emulator,
checks it is still running 20 seconds later, and uploads a screenshot (`demo-ios`,
`demo-android` artifacts).
