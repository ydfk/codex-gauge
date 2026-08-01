<p align="center">
  <img src="native-windows/assets/app-logo.png" width="88" alt="Codex Gauge logo" />
</p>

# Codex Gauge

Codex Gauge 是一个轻量的本机 Codex 用量监控工具。仓库只包含 Windows 与 macOS 两套原生客户端，不需要 Node.js、WebView 或前端构建工具链。

## 平台

| 平台 | 技术 | 形态 | 状态 |
| --- | --- | --- | --- |
| Windows 10/11 x64 | Rust、Slint、windows-rs | 顶部状态条、统一详情/设置面板、系统托盘 | 可开发与发布 |
| macOS 15+ | Swift、SwiftUI、AppKit | 菜单栏状态项与弹出面板 | 可开发；发布需 Apple 签名环境 |

两个客户端分别维护代码、配置和更新机制，但遵循相同的数据与安全边界：优先读取本机 `codex app-server`，不可用时才在后端内存中使用 Codex OAuth 登录状态查询用量；认证字段不会进入 UI、配置、快照或日志。

## 功能

- 显示 5 小时与 7 天额度、重置时间、计划和可用重置次数
- `app-server` 与 AuthJson API 双数据源降级
- 手动刷新、定时刷新和失败退避
- 未登录、接口缺失和字段缺失时优雅降级
- Windows 顶部状态条、托盘、开机启动和手动签名更新
- macOS 菜单栏、登录项和 Sparkle 更新

## Windows 开发

要求 Windows 10/11 与 Rust 1.85+：

```powershell
.\native-windows\scripts\dev.ps1
.\native-windows\scripts\check.ps1
.\native-windows\scripts\build.ps1
```

发布产物位于 `native-windows/dist/`。更多说明见 [native-windows/README.md](native-windows/README.md)。

## macOS 开发

要求 macOS 15+ 与 Xcode 26+：

```bash
cd native-macos
./scripts/build.sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test
```

也可以直接打开 `native-macos/CodexGaugeNative.xcodeproj`。更多说明见 [native-macos/README.md](native-macos/README.md)。

## 自动更新

Windows 使用 `latest.json` 与 Minisign 签名，`v*.*.*` 标签触发 x64 发布。macOS 使用 Sparkle `appcast.xml`、Ed25519 签名以及 Apple Developer ID 签名/公证。两套更新资产互不混用，完整配置见 [发布说明](docs/RELEASE.md)。

## 安全

- 不保存 `access_token`、`refresh_token`、Cookie 或完整 `auth.json`
- 不把认证字段传递到 UI 或日志
- 不上传用量、配置、快照或诊断数据
- 不抓取 ChatGPT 网页
- 不调用重置次数消耗接口

详见 [安全说明](docs/SECURITY.md)。

## 目录

```text
codex-gauge/
├─ native-windows/          # Rust + Slint + windows-rs
├─ native-macos/            # SwiftUI + AppKit + Sparkle
├─ .github/workflows/       # 原生 Windows x64 发布
├─ docs/                    # 设计、安全与发布说明
├─ README.md
├─ RELEASE.md
└─ SECURITY.md
```

## License

本仓库暂未声明开源许可证。在添加许可证前，默认保留全部权利。
