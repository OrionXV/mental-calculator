#!/usr/bin/env bash
# Builds the Rust library for arm64-v8a and x86_64, packages it into a signed APK
# with 16 KB page-aligned native libraries, and prints the verification output.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK_VERSION="${NDK_VERSION:-29.0.14206865}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/$NDK_VERSION}"
if [[ -n "${JAVA_HOME:-}" ]]; then
    export PATH="$JAVA_HOME/bin:$PATH"
fi

PLATFORM_VERSION="${PLATFORM_VERSION:-android-37.2}"
BUILD_TOOLS_VERSION="${BUILD_TOOLS_VERSION:-37.0.0}"
PLATFORM_JAR="$ANDROID_HOME/platforms/$PLATFORM_VERSION/android.jar"
BUILD_TOOLS="$ANDROID_HOME/build-tools/$BUILD_TOOLS_VERSION"
AAPT2="$BUILD_TOOLS/aapt2"
APKSIGNER="$BUILD_TOOLS/apksigner"
ZIPALIGN="$BUILD_TOOLS/zipalign"
KEYTOOL="$(command -v keytool || true)"
if [[ -n "${JAVA_HOME:-}" && -x "$JAVA_HOME/bin/keytool" ]]; then
    KEYTOOL="$JAVA_HOME/bin/keytool"
fi

MIN_SDK="${MIN_SDK:-26}"
TARGET_SDK="${TARGET_SDK:-37}"
LIB_NAME=libmentalcalc_android.so
APK_DIR="$ROOT/target/apk"
STAGE="$APK_DIR/stage"
KEYSTORE="$APK_DIR/debug.keystore"
UNSIGNED="$APK_DIR/unsigned.apk"
ALIGNED="$APK_DIR/aligned.apk"
OUTPUT="$APK_DIR/mentalcalc.apk"

die() {
    echo "error: $*" >&2
    exit 1
}

case "$(uname -s)-$(uname -m)" in
    Darwin-*) NDK_HOST_DEFAULT=darwin-x86_64 ;;
    Linux-x86_64) NDK_HOST_DEFAULT=linux-x86_64 ;;
    *) NDK_HOST_DEFAULT="" ;;
esac
NDK_HOST_TAG="${NDK_HOST_TAG:-$NDK_HOST_DEFAULT}"
[[ -n "$NDK_HOST_TAG" ]] || die "unsupported host; set NDK_HOST_TAG to your NDK prebuilt host directory"
LLVM_STRIP="${LLVM_STRIP:-$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST_TAG/bin/llvm-strip}"

for tool in cargo-ndk "$PLATFORM_JAR" "$AAPT2" "$APKSIGNER" "$ZIPALIGN" "$LLVM_STRIP" "$KEYTOOL" zip unzip; do
    if [[ "$tool" == */* ]]; then
        [[ -e "$tool" ]] || die "missing $tool"
    else
        command -v "$tool" >/dev/null 2>&1 || die "missing $tool"
    fi
done

echo "==> 1/6 cargo ndk build (arm64-v8a, x86_64)"
cargo ndk -t arm64-v8a -t x86_64 build --release -p mentalcalc-android

mkdir -p "$APK_DIR"
rm -rf "$STAGE" "$UNSIGNED" "$ALIGNED" "$OUTPUT"
mkdir -p "$STAGE/lib/arm64-v8a" "$STAGE/lib/x86_64"
cp "$ROOT/target/aarch64-linux-android/release/$LIB_NAME" "$STAGE/lib/arm64-v8a/"
cp "$ROOT/target/x86_64-linux-android/release/$LIB_NAME" "$STAGE/lib/x86_64/"

echo "==> 2/6 aapt2 link manifest"
"$AAPT2" link \
    -I "$PLATFORM_JAR" \
    --manifest "$ROOT/android/AndroidManifest.xml" \
    --min-sdk-version "$MIN_SDK" \
    --target-sdk-version "$TARGET_SDK" \
    -o "$UNSIGNED"

echo "==> 3/6 add native libraries, stripped, stored uncompressed"
"$LLVM_STRIP" --strip-unneeded "$STAGE/lib/arm64-v8a/$LIB_NAME" "$STAGE/lib/x86_64/$LIB_NAME"
(
    cd "$STAGE"
    zip -q -0 "$UNSIGNED" "lib/arm64-v8a/$LIB_NAME" "lib/x86_64/$LIB_NAME"
)

echo "==> 4/6 zipalign 16 KB pages and verify"
"$ZIPALIGN" -P 16 -f 4 "$UNSIGNED" "$ALIGNED"
"$ZIPALIGN" -c -P 16 4 "$ALIGNED"

echo "==> 5/6 debug keystore"
if [[ ! -f "$KEYSTORE" ]]; then
    "$KEYTOOL" -genkeypair \
        -keystore "$KEYSTORE" \
        -storepass android \
        -keypass android \
        -alias androiddebugkey \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -dname "CN=Android Debug,O=Android,C=US"
fi

echo "==> 6/6 apksigner sign and verify"
"$APKSIGNER" sign \
    --ks "$KEYSTORE" \
    --ks-key-alias androiddebugkey \
    --ks-pass pass:android \
    --key-pass pass:android \
    --out "$OUTPUT" \
    "$ALIGNED"
"$APKSIGNER" verify --print-certs "$OUTPUT"

echo "==> badging"
"$AAPT2" dump badging "$OUTPUT" | grep -E "^(package|launchable-activity|sdkVersion|targetSdkVersion|application-label:|native-code)"
echo "==> native libraries"
unzip -l "$OUTPUT" | grep "\.so$"
echo "built $OUTPUT"
