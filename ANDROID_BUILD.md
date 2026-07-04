# Building the Android APK

## What changed for mobile
Android sandboxes apps — no arbitrary shell execution, no picking arbitrary folders like a desktop
app can. So on Android, Frox Code runs as a **plain chat app** talking to your model (no file
editing, no running commands). The sidebar automatically hides the "Open project folder" option
on mobile and shows a note explaining why. Desktop keeps full agent capability. Same codebase,
same UI — it adapts at runtime based on platform.

## Prerequisites
- Android Studio (for the SDK/NDK) — https://developer.android.com/studio
- Rust with Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```
- Set environment variables (adjust paths to match your Android Studio install):
  ```bash
  export ANDROID_HOME=$HOME/Android/Sdk
  export NDK_HOME=$ANDROID_HOME/ndk/$(ls $ANDROID_HOME/ndk)
  ```

## First-time setup
```bash
npm install
npm run tauri android init
```
This generates a `src-tauri/gen/android` folder — the actual Android Studio project. It's
generated, not something I can hand you directly without the Android SDK present.

## Build the APK
Debug build (fastest, for testing on your own device):
```bash
npm run tauri android build -- --debug
```
The APK will be at:
```
src-tauri/gen/android/app/build/outputs/apk/debug/app-universal-debug.apk
```

Release build (needs a signing key — required before sharing publicly or publishing to Play Store):
```bash
npm run tauri android build
```
Follow Tauri's signing guide to generate a keystore first:
https://v2.tauri.app/distribute/sign/android/

## Installing it on a phone
- Enable "Install unknown apps" / "Developer options → USB debugging" on the Android device.
- `adb install path/to/app-universal-debug.apk`, or transfer the file and open it directly.

## If the build fails
Most Android build failures are SDK/NDK version mismatches. Check that `ANDROID_HOME`/`NDK_HOME`
point to real installed versions (`ls $ANDROID_HOME/ndk` should show a version folder), and that
the Rust targets from the prerequisites step installed successfully (`rustup target list --installed`).
Paste me the actual error text and I'll help debug the real cause rather than guessing.
