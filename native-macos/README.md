# Codex Gauge Native

Codex Gauge 的独立原生 macOS 实现。现有 `src/` 与 `src-tauri/` 不参与编译，两个实现可以并行演进。

## 功能

- SwiftUI `MenuBarExtra` 菜单栏应用，不显示 Dock 图标。
- 读取 Codex `app-server` 的 5 小时与 7 天额度。
- 使用本机 `~/.codex/auth.json` 查询 AuthJson API，并作为双向回退数据源。
- 展示剩余额度、重置时间、当前计划、数据来源与重置券明细。
- 支持菜单栏密度、刷新间隔、Codex 命令路径、数据源优先级和登录时启动。
- macOS 26 使用系统 Liquid Glass；macOS 15–25 使用原生 Material 降级。
- 集成 Sparkle 2，支持自动检查、签名更新、安装和重启。

## 环境

- Xcode 26 或更高版本。
- 运行最低版本 macOS 15。
- Liquid Glass 需要 macOS 26。
- Swift Package Manager 会解析 Sparkle 2.9.x；仓库锁定的实际版本见 `Package.resolved`。

## 开发

用 Xcode 打开：

```bash
open CodexGaugeNative.xcodeproj
```

或直接构建：

```bash
./scripts/build.sh
```

运行核心解析测试：

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test
```

未签名调试产物位于：

```text
DerivedData/Build/Products/Debug/Codex Gauge.app
```

## 本机安装

只在自己的 Mac 上使用时，不需要制作 DMG、PKG，也不需要配置公开自动更新。推荐通过 Xcode 生成 Release 版，并将 `Codex Gauge.app` 固定安装到 `/Applications`。

第一次配置 Xcode、导出 Release、覆盖更新和登录启动的完整步骤见 [使用 Xcode 在本机安装与更新](docs/LOCAL_XCODE_INSTALL.md)。

## 自动更新

原生版使用 Sparkle appcast，不复用 Tauri 的 `latest.json`。两种更新清单可以作为同一个 GitHub Release 的独立资源并存：

```text
latest.json   # Tauri 版本
appcast.xml   # Swift 原生版本
```

首次配置：

1. 解析一次依赖，让 Xcode 下载 Sparkle 工具。
2. 执行 `DerivedData/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_keys`。
3. 复制 `Config/Local.xcconfig.example` 为 `Config/Local.xcconfig`，写入生成的公钥。
4. 私钥保留在 macOS 钥匙串或 CI Secret，不提交到仓库。
5. 使用 Developer ID 对 `.app` 签名并公证。
6. 运行 `./scripts/release.sh`，生成更新压缩包和 `appcast.xml`。
7. 将两者上传到 GitHub Release。

完整发布边界见 [docs/RELEASE.md](docs/RELEASE.md)。

## 数据与隐私

- 访问令牌只在请求期间保存在内存中。
- 不会把 `access_token`、`refresh_token`、Cookie 或认证请求头写入配置、快照或日志。
- 配置与最后一次成功快照保存在 `~/Library/Application Support/Codex Gauge Native/`。
- 当刷新失败时继续展示上一次成功结果，并在标题区明确提示失败。

## 目录

```text
native-macos/
├─ CodexGaugeNative.xcodeproj/  # 可直接构建的菜单栏 App 工程
├─ CodexGaugeNative/
│  ├─ App/                      # 生命周期与状态
│  ├─ Core/                     # 数据模型、解析与 Codex 客户端
│  ├─ Services/                 # 存储、登录项、刷新与更新
│  └─ Views/                    # SwiftUI 与 Liquid Glass 界面
├─ Tests/                       # Swift Testing 核心测试
├─ Config/                      # Info.plist 与本地发布配置
├─ docs/                        # 原生版发布说明
└─ scripts/                     # 构建与更新产物脚本
```
