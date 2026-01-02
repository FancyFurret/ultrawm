#!/bin/bash
# Build script for creating macOS .app bundle
# Run this script from the workspace root directory

set -e

# Get the script directory and workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${WORKSPACE_ROOT}"

# Get the version from Cargo.toml
VERSION=$(grep '^version' cli/Cargo.toml | cut -d '"' -f 2)
APP_NAME="UltraWM"
APP_BUNDLE="${APP_NAME}.app"
CONTENTS_DIR="${APP_BUNDLE}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

# Build the release binary
echo "Building release binary..."
cargo build --release --bin ultrawm

# Create .app bundle structure
echo "Creating .app bundle structure..."
rm -rf "${APP_BUNDLE}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copy the binary
BINARY_PATH="target/release/ultrawm"
if [ -f "${BINARY_PATH}" ]; then
    cp "${BINARY_PATH}" "${MACOS_DIR}/${APP_NAME}"
    chmod +x "${MACOS_DIR}/${APP_NAME}"
else
    echo "Error: Binary not found at ${BINARY_PATH}"
    exit 1
fi

# Create Info.plist
cat > "${CONTENTS_DIR}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.fancyfurret.ultrawm</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
EOF

# Code sign the app bundle
# This is required for macOS to properly recognize the app for accessibility permissions
# Using ad-hoc signing (-s -) which doesn't require a developer certificate
echo "Code signing app bundle..."
codesign --force --deep --sign - "${APP_BUNDLE}"echo "Created ${APP_BUNDLE}"
echo "You can now run: open ${APP_BUNDLE}"

