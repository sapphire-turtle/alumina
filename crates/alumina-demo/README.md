# alumina-demo

An app to try alumina's widgets by hand, on the desktops, iOS and Android. It is a
workspace member, built and tested with the rest, and never published.

| Page | What it shows |
|---|---|
| Date & time | The picker buttons (date, period, time, timestamp), a calendar on its own with what it picked, a drum and the time drums, and the calendar beside the drums. Settings: Japanese words, the first day of the week, week numbers, the dots, the cell size, the time's format, and which kinds the calendar picks |
| Markdown | Not yet (#1) |

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
cd crates/alumina-demo/ios
xcodegen generate
open AluminaDemo.xcodeproj   # pick a simulator and Run
```

Or from the command line:

```sh
xcodebuild -project AluminaDemo.xcodeproj -scheme AluminaDemo -sdk iphonesimulator \
  -configuration Debug -derivedDataPath build build
xcrun simctl boot "iPhone 16 Pro"
xcrun simctl install booted build/Build/Products/Debug-iphonesimulator/AluminaDemo.app
xcrun simctl launch --console-pty booted org.sapphire-turtle.alumina-demo
```

The project's pre-build script (`build-rust.sh`) builds the demo's staticlib for the SDK
Xcode targets; `Sources/main.m` hands the main thread to it. For a real device, copy
`Local.xcconfig.example` to `Local.xcconfig` and set your `DEVELOPMENT_TEAM` (and a bundle
id of your own).

## Android emulator

Needs the Android SDK with an NDK (`ANDROID_NDK_HOME`),
[cargo-ndk](https://github.com/bbqsrc/cargo-ndk), Gradle 8.11 or later and JDK 17:

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk
crates/alumina-demo/android/build-rust.sh     # arm64-v8a and x86_64; ABIS=x86_64 for one
gradle -p crates/alumina-demo/android assembleDebug
adb install -r crates/alumina-demo/android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.sapphire_turtle.alumina_demo/.MainActivity
adb logcat -s alumina-demo
```

`build-rust.sh` builds the cdylib and puts it in `app/src/main/jniLibs`; the app is
GameActivity (`MainActivity`), loading it and calling its `android_main`. Open
`crates/alumina-demo/android` in Android Studio to run it from there instead.

A debug build runs because the workspace's `Cargo.toml` builds android-activity without
debug assertions (`[profile.dev.package.android-activity]`): android-activity 0.6 makes a
slice of the keyboard's text from a null pointer before there is any, and the check aborts.

## CI

CI builds and tests the demo on the desktops. Mobile CI runs clippy on it for iOS and
Android, builds the iOS app and the APK, starts each on the simulator and the emulator,
checks it is still running 20 seconds later, and uploads a screenshot (`demo-ios`,
`demo-android` artifacts).
