#!/usr/bin/env bash
set -euo pipefail

OUTPUT_DIR="${1:-target/android/jniLibs}"

echo "============================================================"
echo " Building URAE Algebra Engine for Android via cargo-ndk"
echo "============================================================"

if ! command -v cargo-ndk &> /dev/null; then
    echo "cargo-ndk could not be found. Installing..."
    cargo install cargo-ndk
fi

mkdir -p "$OUTPUT_DIR"

echo "[*] Compiling crates/urae-notebook for Android architectures..."
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$OUTPUT_DIR" build --package urae-notebook --release

echo "[*] Compiling crates/urae-ffi for Android architectures..."
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o "$OUTPUT_DIR" build --package urae-ffi --release

echo "[+] Android build complete! Libraries emitted to: $OUTPUT_DIR"
