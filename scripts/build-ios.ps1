#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Builds algebra-engine (urae-ffi) static libraries for iOS and bundles .xcframework.
.DESCRIPTION
    Compiles crates/urae-ffi as staticlib for physical iOS devices (aarch64-apple-ios)
    and iOS simulators (aarch64-apple-ios-sim, x86_64-apple-ios) for linking into Xcode/Swift projects.
.PARAMETER OutputDir
    Destination directory for compiled iOS binaries and XCFramework. Defaults to "target/ios".
.EXAMPLE
    ./scripts/build-ios.ps1
#>
param(
    [string]$OutputDir = "target/ios"
)

$ErrorActionPreference = "Stop"

Write-Host "============================================================"
Write-Host " Building algebra-engine (urae-ffi) for iOS"
Write-Host "============================================================"

$targets = @("aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios")

$installedTargets = rustup target list --installed
foreach ($target in $targets) {
    if ($installedTargets -notcontains $target) {
        Write-Host "[*] Installing missing target: $target..."
        rustup target add $target
    }
}

New-Item -ItemType Directory -Force -Path "$OutputDir/device" | Out-Null
New-Item -ItemType Directory -Force -Path "$OutputDir/simulator" | Out-Null

foreach ($target in $targets) {
    Write-Host "[*] Compiling crates/urae-ffi for $target..."
    cargo build --package urae-ffi --release --target $target
}

Copy-Item -Force "target/aarch64-apple-ios/release/liburae_ffi.a" "$OutputDir/device/liburae_ffi.a" -ErrorAction SilentlyContinue

if (Get-Command lipo -ErrorAction SilentlyContinue) {
    Write-Host "[*] Assembling universal simulator binary with lipo..."
    lipo -create `
        "target/aarch64-apple-ios-sim/release/liburae_ffi.a" `
        "target/x86_64-apple-ios/release/liburae_ffi.a" `
        -output "$OutputDir/simulator/liburae_ffi.a"
} else {
    Write-Host "[i] lipo not available on this host. Preserving individual simulator slices."
    Copy-Item -Force "target/aarch64-apple-ios-sim/release/liburae_ffi.a" "$OutputDir/simulator/liburae_ffi_arm64.a" -ErrorAction SilentlyContinue
    Copy-Item -Force "target/x86_64-apple-ios/release/liburae_ffi.a" "$OutputDir/simulator/liburae_ffi_x86_64.a" -ErrorAction SilentlyContinue
}

if (Get-Command xcodebuild -ErrorAction SilentlyContinue) {
    Write-Host "[*] Generating UraeFFI.xcframework via xcodebuild..."
    $xcframeworkPath = "$OutputDir/UraeFFI.xcframework"
    if (Test-Path $xcframeworkPath) {
        Remove-Item -Recurse -Force $xcframeworkPath
    }
    xcodebuild -create-xcframework `
        -library "$OutputDir/device/liburae_ffi.a" `
        -library "$OutputDir/simulator/liburae_ffi.a" `
        -output $xcframeworkPath
    Write-Host "[+] Generated $xcframeworkPath"
} else {
    Write-Host "[i] xcodebuild not available on this host. Static libraries ready for Swift Package Manager import."
}

Write-Host "[+] iOS build completed successfully! Output: $OutputDir"
