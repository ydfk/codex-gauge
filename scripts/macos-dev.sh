#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_PATH="$PROJECT_ROOT/src-tauri/target/debug/bundle/macos/Codex Gauge.app"
APP_BINARY="$APP_PATH/Contents/MacOS/codex-gauge"
APP_PLIST="$APP_PATH/Contents/Info.plist"

cd "$PROJECT_ROOT"

pass() {
  printf '[OK] %s\n' "$1"
}

warn() {
  printf '[WARN] %s\n' "$1" >&2
}

fail() {
  printf '[FAIL] %s\n' "$1" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "缺少命令：$1"
}

check_environment() {
  [[ "$(uname -s)" == "Darwin" ]] || fail "该脚本只能在 macOS 下运行"
  [[ "$(uname -m)" == "arm64" ]] || fail "当前仅支持 Apple Silicon（arm64）"
  pass "macOS Apple Silicon"

  if ! command -v cargo >/dev/null 2>&1 && [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
    export PATH="${HOME}/.cargo/bin:$PATH"
  fi

  require_command node
  require_command pnpm
  require_command cargo
  require_command rustc
  require_command xcrun
  xcrun --find clang >/dev/null 2>&1 || fail "未安装 Xcode Command Line Tools"

  local node_major
  local pnpm_major
  local rust_host
  node_major="$(node -p 'process.versions.node.split(".")[0]')"
  pnpm_major="$(pnpm --version | cut -d. -f1)"
  rust_host="$(rustc -vV | sed -n 's/^host: //p')"

  ((node_major >= 26)) || fail "Node.js 需要 26+，当前为 $(node --version)"
  ((pnpm_major >= 11)) || fail "pnpm 需要 11+，当前为 $(pnpm --version)"
  [[ "$rust_host" == "aarch64-apple-darwin" ]] || fail "Rust host 应为 aarch64-apple-darwin，当前为 $rust_host"

  pass "Node.js $(node --version)"
  pass "pnpm $(pnpm --version)"
  pass "$(rustc --version)"
  pass "Xcode Command Line Tools"

  if command -v codex >/dev/null 2>&1; then
    pass "Codex CLI：$(command -v codex)"
  else
    warn "未在 PATH 中找到 Codex CLI；仍可测试未登录状态和界面"
  fi
}

check_dependencies() {
  [[ -d "$PROJECT_ROOT/node_modules" ]] || fail "依赖尚未安装，请先运行 pnpm install"
  pass "前端依赖已安装"
}

verify_bundle() {
  [[ -x "$APP_BINARY" ]] || fail "没有找到调试应用：$APP_PATH"
  [[ -f "$APP_PLIST" ]] || fail "应用缺少 Info.plist"

  file "$APP_BINARY" | grep -q 'arm64' || fail "应用二进制不是 arm64"
  [[ "$(plutil -extract LSUIElement raw -o - "$APP_PLIST")" == "true" ]] \
    || fail "LSUIElement 未启用，应用可能显示 Dock 图标"
  [[ "$(plutil -extract LSMinimumSystemVersion raw -o - "$APP_PLIST")" == "12.0" ]] \
    || fail "最低系统版本不是 macOS 12.0"

  pass "调试应用为 arm64"
  pass "菜单栏应用配置正确（LSUIElement=true）"
  pass "调试应用：$APP_PATH"
}

print_manual_checks() {
  printf '\n手动检查清单：\n'
  printf '  1. 菜单栏显示与 Windows 一致的彩色托盘图标和 5h/7d，Dock 中没有应用图标。\n'
  printf '  2. 单击菜单栏状态项可打开详情，再次单击、失焦或按 Esc 可关闭。\n'
  printf '  3. 详情中的 5h、7d、重置次数和刷新功能可正常使用。\n'
  printf '  4. 设置页的显示模式、刷新间隔和登录时启动可以保存。\n'
  printf '  5. 深色/浅色外观与多显示器下的弹出位置正常。\n'
}

run_doctor() {
  check_environment
  printf '\n开发环境检查通过。\n'
}

run_dev() {
  check_environment
  check_dependencies
  print_manual_checks
  printf '\n正在启动 Tauri 开发模式，按 Ctrl+C 停止。\n'
  exec pnpm tauri dev
}

run_test() {
  check_environment
  check_dependencies
  pnpm check
  pnpm tauri build --debug --bundles app
  verify_bundle
  print_manual_checks
}

run_open() {
  check_environment
  verify_bundle
  open -n "$APP_PATH"
  print_manual_checks
}

print_help() {
  cat <<'EOF'
用法：bash scripts/macos-dev.sh <命令>

命令：
  doctor  检查 Apple Silicon、Node、pnpm、Rust 和 Xcode 工具链
  dev     启动 macOS 菜单栏应用开发模式
  test    运行前端/Rust 检查并构建、校验调试版 .app
  open    打开 test 已生成的 .app，进行菜单栏交互测试
EOF
}

case "${1:-help}" in
  doctor) run_doctor ;;
  dev) run_dev ;;
  test) run_test ;;
  open) run_open ;;
  help|-h|--help) print_help ;;
  *)
    print_help >&2
    fail "未知命令：$1"
    ;;
esac
