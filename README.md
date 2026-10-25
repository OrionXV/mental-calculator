# Mental Calculator (Rust)

A mental arithmetic trainer for Android, written in Rust. Pick how long you want to practise, how hard the numbers should be, and which operations to include. Then solve as many problems as you can before the timer runs out.

I used the original app a lot when I was bored, and this is a rewrite of it for current Android. The original was a 2014 Russian-language app. This version keeps its core rules and drops the ads, analytics, and reminder notifications.

## Download

Get the latest signed APK from the [releases page](https://github.com/OrionXV/mental-calculator/releases/latest). Open the file on your Android phone and allow installs from this source when asked. It works on Android 8.0 and up.

## Features

- **Timed sessions** from 1 to 20 minutes, with pause and resume.
- **Four difficulty levels**, each with its own operand ranges:

  | Level  | Operands                            |
  |--------|-------------------------------------|
  | Easy   | both 2 to 9                         |
  | Medium | 10 to 99 and 2 to 9                 |
  | Hard   | both 10 to 99                       |
  | Expert | both 100 to 999                     |

- **Four operations**, any combination: addition, subtraction, division, and multiplication. Division problems always come out exact, and subtraction never goes negative.
- **On-screen keypad** for answers. A wrong answer is counted and the problem is skipped. Empty input is ignored.
- **Results** after each session: percent correct, a grade (below 20%, 20 to 39%, 40 to 69%, 70% and up), and speed in answers per planned minute.
- **Archive** of past sessions, newest first, with a button to clear it.
- **English and Russian** interface. English is the default, and you can switch in Settings.
- **Plain-text data.** Settings are kept in `settings.txt`, and the session archive in `sessions_results.mcd`, using the same line format as the original app.

Not ported: the morning reminder notification, ads, and analytics.

## Project layout

- `crates/mentalcalc-core` is the platform-independent logic: problem generation, the session timer, scoring, the archive format, and settings. It has no Android dependencies and is covered by unit tests.
- `crates/mentalcalc-android` is the egui user interface. It runs on Android through `NativeActivity` and on desktop through an example runner.
- `android/AndroidManifest.xml` is the Android manifest.
- `scripts/build-apk.sh` builds, aligns, and signs the APK.

## Building

### Prerequisites

1. **Rust** (stable) from [rustup](https://rustup.rs).
2. **Android Rust targets** and **cargo-ndk**:

   ```bash
   rustup target add aarch64-linux-android x86_64-linux-android
   ```

   ```bash
   cargo install cargo-ndk
   ```

3. **JDK 17 or newer** (JDK 21 was used). The build script uses its `keytool`.
4. **Android SDK** with these packages, installed with `sdkmanager`:
   - `platforms;android-37.2`
   - `build-tools;37.0.0`
   - `ndk;29.0.14206865`
   - `platform-tools`

   Accept the SDK licences with `sdkmanager --licenses` before installing.

Set `ANDROID_HOME` to your SDK location. The default is `$HOME/Library/Android/sdk`, the macOS location.

### Run and test on desktop

The desktop runner shows the same screens in a window with a phone-sized layout:

```bash
cargo run -p mentalcalc-android --example desktop -- --screen main
```

Available screens are `main`, `session-settings`, `session`, `result`, `archive`, and `settings`. Add `--screenshot out.png` to save a screenshot and exit, or `--data-dir DIR` to choose where settings and the archive are stored.

Run the tests for the whole workspace:

```bash
cargo test --workspace
```

### Build the Android APK

```bash
scripts/build-apk.sh
```

The signed APK is written to `target/apk/mentalcalc.apk`. The script builds for `arm64-v8a` and `x86_64`, strips the native libraries, and aligns them to 16 KB pages.

To use a different SDK or NDK, set any of these environment variables: `ANDROID_HOME`, `ANDROID_NDK_HOME`, `NDK_VERSION`, `PLATFORM_VERSION`, `BUILD_TOOLS_VERSION`, `MIN_SDK`, `TARGET_SDK`, `NDK_HOST_TAG`, and `LLVM_STRIP`.

### Install on a device or emulator

```bash
adb install -r target/apk/mentalcalc.apk
```

## Notes

- The APK is signed with a debug key that the build script creates locally in `target/apk/`. Because each machine makes its own key, an APK built on one machine cannot update one built on another. For a release build you would need your own signing key.
- The app targets Android 17 (API 37) and supports Android 8.0 and up (API 26).
- Tested on an Android 17 emulator (arm64). The `x86_64` build is produced but not tested. Landscape layout works with scrolling, but only partly tested.
- The system Back key is delivered to the app through a manifest flag. On the session screen it asks before ending the session, and on other screens it returns to the main menu.

## License

MIT. See [LICENSE](LICENSE).
