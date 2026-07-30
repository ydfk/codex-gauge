#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
DERIVED_DATA="$PROJECT_DIR/DerivedData"
ARCHIVE_PATH="$PROJECT_DIR/build/CodexGaugeNative.xcarchive"
OUTPUT_DIR="$PROJECT_DIR/build/update"
APPCAST_TOOL="$DERIVED_DATA/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_appcast"
export DEVELOPER_DIR

if [[ ! -f "$PROJECT_DIR/Config/Local.xcconfig" ]]; then
  echo "Missing Config/Local.xcconfig. Configure the Sparkle public key first." >&2
  exit 1
fi

if [[ ! -x "$APPCAST_TOOL" ]]; then
  xcodebuild \
    -project "$PROJECT_DIR/CodexGaugeNative.xcodeproj" \
    -scheme CodexGaugeNative \
    -derivedDataPath "$DERIVED_DATA" \
    -resolvePackageDependencies
fi

mkdir -p "$OUTPUT_DIR"

xcodebuild \
  -project "$PROJECT_DIR/CodexGaugeNative.xcodeproj" \
  -scheme CodexGaugeNative \
  -configuration Release \
  -derivedDataPath "$DERIVED_DATA" \
  -archivePath "$ARCHIVE_PATH" \
  archive

APP_PATH="$ARCHIVE_PATH/Products/Applications/Codex Gauge.app"
VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")"
ZIP_PATH="$OUTPUT_DIR/Codex-Gauge-Native-$VERSION-macOS.zip"

ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$ZIP_PATH"
"$APPCAST_TOOL" "$OUTPUT_DIR"

echo "Release artifacts: $OUTPUT_DIR"
