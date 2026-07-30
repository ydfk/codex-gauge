#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
export DEVELOPER_DIR

xcodebuild \
  -project "$PROJECT_DIR/CodexGaugeNative.xcodeproj" \
  -scheme CodexGaugeNative \
  -configuration Debug \
  -derivedDataPath "$PROJECT_DIR/DerivedData" \
  CODE_SIGNING_ALLOWED=NO \
  build

echo "Built: $PROJECT_DIR/DerivedData/Build/Products/Debug/Codex Gauge.app"

