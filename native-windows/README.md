# Codex Gauge for Windows

Codex Gauge 的 Windows 原生客户端，使用 Rust、Slint 和 windows-rs，仅支持 Windows 10/11 x64。

## 功能

- 22px 紧凑顶部状态条，支持横向拖动、置顶、锁定与位置保存
- 单击打开统一详情/设置面板
- 系统托盘与右键快捷操作
- 优先通过 `codex app-server` 查询，失败时回退本机 AuthJson API
- 通过 API 查询可用重置次数和重置券详情
- 开机启动、透明度、刷新间隔和 OLED 微位移
- Minisign 校验的手动更新；只有用户点击后才安装

认证信息只在进程内存中用于本机请求。程序不会保存或向 Slint UI 传递 Token、Cookie、完整认证文件或原始服务响应。

## 本地开发

要求 Rust 1.85 或更高版本：

```powershell
.\native-windows\scripts\dev.ps1
.\native-windows\scripts\check.ps1
.\native-windows\scripts\build.ps1
```

发布版输出到 `native-windows/dist/Codex.Gauge.Native_x64.portable.exe`。

本地数据位于 `%APPDATA%\CodexGaugeNative`：

- `config.json`：界面、刷新、数据源和更新设置
- `state.json`：脱敏后的最后快照
- `update.log`：更新操作与泛化错误类别

## 发布

工作流 [release.yml](../.github/workflows/release.yml) 只处理 `v*.*.*` 标签：

```powershell
git tag v0.2.0
git push origin v0.2.0
```

应用内更新需要配置：

- Secret `NATIVE_WINDOWS_SIGNING_PRIVATE_KEY`
- Secret `NATIVE_WINDOWS_SIGNING_PRIVATE_KEY_PASSWORD`，无密码时可留空
- Variable 或 Secret `NATIVE_WINDOWS_UPDATER_PUBKEY`

这些配置不是构建和发布 EXE 的前置条件。未配置时，CI 仍发布便携 EXE，只跳过签名和 `latest.json`；配置完整时，`tools/sign-update` 生成标准 Minisign 签名，最新正式 Release 同时保存程序、签名和更新清单。

## 边界

- 不构建 macOS 或 Linux
- 不依赖 WebView 运行时
- 不自动安装更新
- 不上传用量、设置、日志或认证数据
