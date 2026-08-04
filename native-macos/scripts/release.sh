#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
DERIVED_DATA="${DERIVED_DATA:-$PROJECT_DIR/DerivedData}"
ARCHIVE_PATH="${ARCHIVE_PATH:-$PROJECT_DIR/build/CodexGaugeNative.xcarchive}"
OUTPUT_DIR="${OUTPUT_DIR:-$PROJECT_DIR/build/update}"
APPCAST_TOOL="$DERIVED_DATA/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_appcast"
SIGNING_IDENTITY="${MACOS_SIGNING_IDENTITY:-Developer ID Application}"
RELEASE_VERSION="${RELEASE_VERSION:-}"
RELEASE_BUILD_NUMBER="${RELEASE_BUILD_NUMBER:-}"
RELEASE_TAG="${RELEASE_TAG:-}"
DOWNLOAD_URL_PREFIX="${DOWNLOAD_URL_PREFIX:-}"
RELEASE_COMPLETED=0
export DEVELOPER_DIR

require_value() {
  local name="$1"
  local value="${!name:-}"

  if [[ -z "$value" ]]; then
    echo "缺少必需的环境变量：$name" >&2
    exit 1
  fi
}

cleanup() {
  local exit_status=$?

  set +e

  if [[ -n "${DMG_STAGING_DIR:-}" && -d "$DMG_STAGING_DIR" ]]; then
    rm -rf "$DMG_STAGING_DIR"
  fi

  if [[ -n "${EXPORT_WORK_DIR:-}" && -d "$EXPORT_WORK_DIR" ]]; then
    rm -rf "$EXPORT_WORK_DIR"
  fi

  if [[ "$RELEASE_COMPLETED" != "1" && "$exit_status" -eq 0 ]]; then
    exit_status=1
  fi

  exit "$exit_status"
}

verify_distribution_signature() {
  local target="$1"
  local label="$2"
  local signature_details
  local signed_team

  codesign --strict --verify --verbose=2 "$target"
  signature_details="$(codesign --display --verbose=4 "$target" 2>&1)"
  signed_team="$(printf '%s\n' "$signature_details" | awk -F= '/^TeamIdentifier=/{print $2; exit}')"

  if [[ "$signed_team" != "$APPLE_TEAM_ID" ]]; then
    echo "$label 签名团队 $signed_team 与 APPLE_TEAM_ID 不一致。" >&2
    exit 1
  fi

  if ! printf '%s\n' "$signature_details" | grep -q '^Authority=Developer ID Application:'; then
    echo "$label 未使用 Developer ID Application 证书签名。" >&2
    exit 1
  fi

  if ! printf '%s\n' "$signature_details" | grep -Eq '^Timestamp=.+$' ||
    printf '%s\n' "$signature_details" | grep -Eq '^Timestamp=(none|0)$'; then
    echo "$label 签名不包含安全时间戳。" >&2
    exit 1
  fi

  if ! printf '%s\n' "$signature_details" | grep -Eq '^CodeDirectory .*flags=.*runtime'; then
    echo "$label 签名未启用 Hardened Runtime。" >&2
    exit 1
  fi
}

trap cleanup EXIT

require_value APPLE_TEAM_ID
require_value APPLE_NOTARY_API_KEY_PATH
require_value APPLE_NOTARY_KEY_ID
require_value APPLE_NOTARY_ISSUER_ID

if [[ ! -f "$PROJECT_DIR/Config/Local.xcconfig" ]]; then
  echo "缺少 Config/Local.xcconfig，请先配置 Sparkle 公钥。" >&2
  exit 1
fi

if [[ ! -f "$APPLE_NOTARY_API_KEY_PATH" ]]; then
  echo "找不到 Apple 公证 API Key：$APPLE_NOTARY_API_KEY_PATH" >&2
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

BUILD_SETTINGS=(
  "ARCHS=arm64"
  "ONLY_ACTIVE_ARCH=NO"
  "CODE_SIGN_STYLE=Manual"
  "CODE_SIGN_IDENTITY=$SIGNING_IDENTITY"
  "DEVELOPMENT_TEAM=$APPLE_TEAM_ID"
)

if [[ -n "$RELEASE_VERSION" ]]; then
  BUILD_SETTINGS+=("MARKETING_VERSION=$RELEASE_VERSION")
fi

if [[ -n "$RELEASE_BUILD_NUMBER" ]]; then
  BUILD_SETTINGS+=("CURRENT_PROJECT_VERSION=$RELEASE_BUILD_NUMBER")
fi

xcodebuild \
  -project "$PROJECT_DIR/CodexGaugeNative.xcodeproj" \
  -scheme CodexGaugeNative \
  -configuration Release \
  -destination "generic/platform=macOS" \
  -derivedDataPath "$DERIVED_DATA" \
  -archivePath "$ARCHIVE_PATH" \
  archive \
  "${BUILD_SETTINGS[@]}"

EXPORT_WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-gauge-export.XXXXXX")"
EXPORT_OPTIONS_PATH="$EXPORT_WORK_DIR/ExportOptions.plist"
EXPORT_PATH="$EXPORT_WORK_DIR/export"

plutil -create xml1 "$EXPORT_OPTIONS_PATH"
plutil -insert destination -string export "$EXPORT_OPTIONS_PATH"
plutil -insert method -string developer-id "$EXPORT_OPTIONS_PATH"
plutil -insert signingCertificate -string "$SIGNING_IDENTITY" "$EXPORT_OPTIONS_PATH"
plutil -insert signingStyle -string manual "$EXPORT_OPTIONS_PATH"
plutil -insert stripSwiftSymbols -bool true "$EXPORT_OPTIONS_PATH"
plutil -insert teamID -string "$APPLE_TEAM_ID" "$EXPORT_OPTIONS_PATH"

xcodebuild \
  -exportArchive \
  -archivePath "$ARCHIVE_PATH" \
  -exportPath "$EXPORT_PATH" \
  -exportOptionsPlist "$EXPORT_OPTIONS_PATH"

APP_PATH="$EXPORT_PATH/Codex Gauge.app"
VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_PATH/Contents/Info.plist")"
BUILD_NUMBER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP_PATH/Contents/Info.plist")"
SPARKLE_PUBLIC_KEY="$(/usr/libexec/PlistBuddy -c 'Print :SUPublicEDKey' "$APP_PATH/Contents/Info.plist")"
DMG_PATH="$OUTPUT_DIR/Codex-Gauge-Native-$VERSION-macOS-arm64.dmg"

if [[ -n "$RELEASE_TAG" && "$RELEASE_TAG" != "v$VERSION" ]]; then
  echo "发布标签 $RELEASE_TAG 与应用版本 v$VERSION 不一致。" >&2
  exit 1
fi

if ! lipo -archs "$APP_PATH/Contents/MacOS/CodexGaugeNative" | grep -qx "arm64"; then
  echo "发布构建不是仅包含 arm64 的应用。" >&2
  exit 1
fi

if ! PUBLIC_KEY_BYTES="$(printf '%s' "$SPARKLE_PUBLIC_KEY" | base64 -D 2>/dev/null | wc -c | tr -d ' ')"; then
  PUBLIC_KEY_BYTES=0
fi
if [[ "$PUBLIC_KEY_BYTES" != "32" ]]; then
  echo "应用中的 SUPublicEDKey 不是有效的 Sparkle Ed25519 公钥。" >&2
  exit 1
fi

codesign --deep --strict --verify --verbose=2 "$APP_PATH"
SPARKLE_FRAMEWORK="$APP_PATH/Contents/Frameworks/Sparkle.framework"
SPARKLE_VERSION="$SPARKLE_FRAMEWORK/Versions/B"

verify_distribution_signature "$SPARKLE_VERSION/Updater.app" "Sparkle Updater"
verify_distribution_signature "$SPARKLE_VERSION/Autoupdate" "Sparkle Autoupdate"
verify_distribution_signature "$SPARKLE_VERSION/XPCServices/Downloader.xpc" "Sparkle Downloader"
verify_distribution_signature "$SPARKLE_VERSION/XPCServices/Installer.xpc" "Sparkle Installer"
verify_distribution_signature "$SPARKLE_FRAMEWORK" "Sparkle framework"
verify_distribution_signature "$APP_PATH" "应用"

DMG_STAGING_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-gauge-dmg.XXXXXX")"
ditto "$APP_PATH" "$DMG_STAGING_DIR/Codex Gauge.app"
ln -s /Applications "$DMG_STAGING_DIR/Applications"

hdiutil create \
  -volname "Codex Gauge" \
  -srcfolder "$DMG_STAGING_DIR" \
  -fs APFS \
  -format ULFO \
  -ov \
  "$DMG_PATH"

codesign --force --sign "$SIGNING_IDENTITY" --timestamp "$DMG_PATH"
codesign --verify --verbose=2 "$DMG_PATH"

NOTARY_SUBMISSION_PATH="$OUTPUT_DIR/notarytool-submit.json"
NOTARY_LOG_PATH="$OUTPUT_DIR/notarization-log.json"

rm -f "$NOTARY_SUBMISSION_PATH" "$NOTARY_LOG_PATH"

xcrun notarytool submit "$DMG_PATH" \
  --key "$APPLE_NOTARY_API_KEY_PATH" \
  --key-id "$APPLE_NOTARY_KEY_ID" \
  --issuer "$APPLE_NOTARY_ISSUER_ID" \
  --wait \
  --timeout 60m \
  --output-format json > "$NOTARY_SUBMISSION_PATH"

if ! NOTARY_SUBMISSION_ID="$(plutil -extract id raw -o - "$NOTARY_SUBMISSION_PATH")" ||
  ! NOTARY_STATUS="$(plutil -extract status raw -o - "$NOTARY_SUBMISSION_PATH")"; then
  echo "无法解析 Apple 公证提交结果：" >&2
  cat "$NOTARY_SUBMISSION_PATH" >&2
  exit 1
fi

echo "Apple 公证提交：${NOTARY_SUBMISSION_ID}（${NOTARY_STATUS}）"

if [[ "$NOTARY_STATUS" != "Accepted" ]]; then
  echo "Apple 公证未通过，正在获取详细日志。" >&2

  if xcrun notarytool log \
    --key "$APPLE_NOTARY_API_KEY_PATH" \
    --key-id "$APPLE_NOTARY_KEY_ID" \
    --issuer "$APPLE_NOTARY_ISSUER_ID" \
    "$NOTARY_SUBMISSION_ID" \
    "$NOTARY_LOG_PATH"; then
    cat "$NOTARY_LOG_PATH" >&2
  else
    echo "获取 Apple 公证日志失败，可使用提交 ID 手动查询。" >&2
  fi

  exit 1
fi

xcrun stapler staple "$DMG_PATH"
xcrun stapler validate "$DMG_PATH"
spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG_PATH"

APPCAST_ARGUMENTS=(
  --maximum-versions 1
)

if [[ -n "$DOWNLOAD_URL_PREFIX" ]]; then
  APPCAST_ARGUMENTS+=(--download-url-prefix "$DOWNLOAD_URL_PREFIX")
fi

if [[ -n "${SPARKLE_PRIVATE_KEY:-}" ]]; then
  printf '%s' "$SPARKLE_PRIVATE_KEY" |
    "$APPCAST_TOOL" --ed-key-file - "${APPCAST_ARGUMENTS[@]}" "$OUTPUT_DIR"
else
  "$APPCAST_TOOL" "${APPCAST_ARGUMENTS[@]}" "$OUTPUT_DIR"
fi

if [[ ! -f "$OUTPUT_DIR/appcast.xml" ]]; then
  echo "Sparkle appcast.xml 未生成。" >&2
  exit 1
fi

if ! grep -q 'sparkle:edSignature=' "$OUTPUT_DIR/appcast.xml"; then
  echo "Sparkle appcast.xml 不包含更新包签名。" >&2
  exit 1
fi

echo "发布版本：$VERSION ($BUILD_NUMBER)"
echo "发布架构：arm64"
echo "发布产物：$DMG_PATH"
echo "更新清单：$OUTPUT_DIR/appcast.xml"

RELEASE_COMPLETED=1
