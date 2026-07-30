# Codex Gauge Native

Codex Gauge 的独立 Windows 原生实现，使用 Rust、Slint 和 windows-rs。它与仓库根目录的 Tauri 客户端隔离，不共享构建产物或本地配置。

## 功能

- 顶部状态条，支持横向拖动、悬停详情和双击打开完整信息
- 系统托盘、详细信息和独立设置窗口
- 顶部状态条可独立控制置顶、锁定和位置保存
- 优先通过 `codex app-server` 查询，失败时回退本机 AuthJson API
- 仅通过 API 查询可用重置次数
- 开机启动、透明度、刷新间隔和 OLED 微位移
- Windows x64 签名更新，检查后由用户手动确认安装

认证信息只在进程内存中用于本机请求。程序不会保存或向 Slint UI 传递 token、Cookie、完整认证文件或原始服务响应。

## 本地开发

环境要求：Windows 10/11、Rust 1.85 或更高版本。

```powershell
.\native-windows\scripts\dev.ps1
.\native-windows\scripts\check.ps1
.\native-windows\scripts\build.ps1
```

发布版输出到 `native-windows/dist/Codex.Gauge.Native_x64.portable.exe`。本地配置位于 `%APPDATA%\CodexGaugeNative`，不会覆盖 Tauri 版本的数据。更新检查与安装失败的脱敏记录位于 `%APPDATA%\CodexGaugeNative\update.log`。

## 原生版发布

工作流 [native-windows-release.yml](../.github/workflows/native-windows-release.yml) 只处理 `native-v*.*.*` 标签，例如：

```powershell
git tag native-v0.1.0
git push origin native-v0.1.0
```

仓库需要配置：

- Secret `TAURI_SIGNING_PRIVATE_KEY`
- Secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，无密码时可留空
- Variable 或 Secret `TAURI_UPDATER_PUBKEY`

工作流只构建 Windows x64 便携 EXE。版本 Release 保存程序和签名，固定的 `native-latest` Release 保存 `latest-native-windows.json`。客户端默认从该清单检查更新，并只接受签名验证通过的 `windows-x86_64` 资产。

## 隔离边界

- 不构建 macOS 或 Linux
- 不依赖 Electron、Tauri 或 WebView 运行时
- 不修改现有 Tauri 客户端
- 不自动安装更新；只有用户点击安装后才执行
- 不上传用量、设置、日志或认证数据
