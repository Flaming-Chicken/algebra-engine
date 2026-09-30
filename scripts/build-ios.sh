#!/usr/bin/env bash
set -euo pipefail

OUTPUT_DIR="${1:-target/ios}"

echo "============================================================"
echo " Building algebra-engine (urae-ffi) for iOS"
echo "============================================================"

TARGETS=("aarch64-apple-ios" "aarch64-apple-ios-sim" "x86_64-apple-ios")

for target in "${TARGETS[@]}"; do
    if ! rustup target list --installed | grep -q "^${target}\$"; then
        echo "[*] Installing missing target: ${target}..."
        rustup target add "$target"
    fi
done

mkdir -p "$OUTPUT_DIR/device"
mkdir -p "$OUTPUT_DIR/simulator"

for target in "${TARGETS[@]}"; do
    echo "[*] Compiling crates/urae-ffi for ${target}..."
    cargo build --package urae-ffi --release --target "$target"
done

cp -f target/aarch64-apple-ios/release/liburae_ffi.a "$OUTPUT_DIR/device/liburae_ffi.a" 2>/dev/null || true

if command -v lipo &> /dev/null; then
    echo "[*] Assembling universal simulator binary with lipo..."
    lipo -create \
        target/aarch64-apple-ios-sim/release/liburae_ffi.a \
        target/x86_64-apple-ios/release/liburae_ffi.a \
        -output "$OUTPUT_DIR/simulator/liburae_ffi.a"
else
    echo "[i] lipo not available. Preserving individual simulator slices."
    cp -f target/aarch64-apple-ios-sim/release/liburae_ffi.a "$OUTPUT_DIR/simulator/liburae_ffi_arm64.a" 2>/dev/null || true
    cp -f target/x86_64-apple-ios/release/liburae_ffi.a "$OUTPUT_DIR/simulator/liburae_ffi_x86_64.a" 2>/dev/null || true
fi

if command -v xcodebuild &> /dev/null; then
    echo "[*] Generating UraeFFI.xcframework via xcodebuild..."
    rm -rf "$OUTPUT_DIR/UraeFFI.xcframework"
    xcodebuild -create-xcframework \
        -library "$OUTPUT_DIR/device/liburae_ffi.a" \
        -library "$OUTPUT_DIR/simulator/liburae_ffi.a" \
        -output "$OUTPUT_DIR/UraeFFI.xcframework"
    echo "[+] Generated $OUTPUT_DIR/UraeFFI.xcframework"
fi

echo "[+] iOS build completed successfully! Output: $OUTPUT_DIR"
