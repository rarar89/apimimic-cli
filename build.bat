#!/bin/bash

# Clean previous builds
cargo clean

# Build for each platform
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin

# Create release directories
mkdir -p releases

# Package Windows release
cp target/x86_64-pc-windows-msvc/release/apimimic.exe releases/apimimic-windows.exe

# Package Linux release
cp target/x86_64-unknown-linux-gnu/release/apimimic releases/apimimic-linux

# Package macOS release
cp target/x86_64-apple-darwin/release/apimimic releases/apimimic-macos