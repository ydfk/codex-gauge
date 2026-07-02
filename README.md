# Codex Gauge

Codex Gauge 是一个轻量级 Windows 桌面浮窗，用于通过本机 `codex app-server` 查看 Codex 用量。它不读取 Codex 认证文件，不保存 Token，也不上传任何用户数据。

## Features

- Tauri v2 + Svelte + TypeScript + Rust
- Windows 透明无边框浮窗，默认约 `220x92`
- 系统托盘常驻，支持打开、隐藏、刷新、设置和退出
- 通过 stdio JSON-RPC 调用本机 `codex app-server`
- 显示 5 小时窗口、一周窗口、可用重置次数和 Token 统计
- 本地 JSON 保存配置、状态和最多 90 天用量历史
- GitHub Releases 在线检查更新，支持手动安装
- Windows x64 GitHub Actions 打包发布

## Security

Codex Gauge 遵守这些边界：

- 不读取 `~/.codex/auth.json`
- 不保存 `access_token`、`refresh_token`、Cookie 或任何敏感凭据
- 不抓取 ChatGPT 网页
- 不上传账号、用量、配置或历史数据
- 不输出 app-server 原始响应、账号邮箱明文或 Token
- 不调用消耗重置次数的接口

更多说明见 [SECURITY.md](SECURITY.md)。

## Requirements

- Windows 10/11 x64
- Node.js 26+
- pnpm 11+
- Rust stable
- Codex CLI，并可在本机启动 `codex app-server`

## Development

```bash
pnpm install
pnpm dev:desktop
```

常用脚本：

```bash
pnpm dev              # 仅启动 Vite 前端
pnpm build            # 前端类型检查与构建
pnpm check            # 前端 + Rust fmt/check/test
pnpm rust:check       # Rust fmt/check/test
pnpm rust:fmt         # 格式化 Rust
pnpm tauri:dev        # 启动桌面应用
pnpm tauri:build      # 本地 Windows 打包
pnpm diagnose:codex   # 脱敏诊断 Codex CLI/app-server
pnpm version:sync v0.1.0
```

## Troubleshooting Unknown Data

如果界面一直显示“未知”，通常是下面几类原因：

1. `codex` 命令不可执行或被 Windows 拒绝访问。
2. `codex app-server` 没有启动成功。
3. 当前 Codex 没登录，或登录态不可被 app-server 使用。
4. 当前 Codex CLI 版本不支持 `account/rateLimits/read` 或 `account/usage/read`。
5. app-server 返回字段结构变化，解析器只能降级显示“未知”。

先运行脱敏诊断：

```bash
pnpm diagnose:codex
```

诊断脚本不会读取 `~/.codex/auth.json`，也不会输出 app-server 原始响应。重点看：

- `codex --version` 是否 `[OK]`
- `initialize / initialized` 是否 `[OK]`
- `account/read` 是否 `[OK]`
- `account/rateLimits/read` 是否 `[OK]`
- `account/usage/read` 失败时只会影响 Token 统计，不应影响 5h/1w 主用量

如果 `codex --version` 是 `permission_denied`，请检查 Codex 安装来源、WindowsApps 权限，或在设置里把 Codex command 改为可执行的真实路径。

## Local Data

Windows 默认路径：

- `%APPDATA%\CodexGauge\config.json`
- `%APPDATA%\CodexGauge\state.json`
- `%APPDATA%\CodexGauge\usage-history.json`

这些文件只保存配置、脱敏状态和本地统计，不包含 OAuth Token、Cookie 或 app-server 原始响应。

## Release

GitHub Actions 在 SemVer tag 上打包 Windows x64：

```bash
git tag v0.1.0
git push origin v0.1.0
```

发布前需要在 GitHub 仓库中配置：

- Repository variable: `TAURI_UPDATER_PUBKEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，如果私钥设置了密码

生成 updater 密钥：

```bash
pnpm tauri signer generate --write-keys updater.key
```

`src-tauri/tauri.conf.json` 默认使用：

```text
https://github.com/liyuhang/codex-gauge/releases/latest/download/latest.json
```

如果你的 GitHub 仓库不是 `liyuhang/codex-gauge`，请在源码中替换该 endpoint；CI 发布时会自动改为当前 `${{ github.repository }}`。

## Update Flow

应用设置页提供“检查更新”和“安装更新”。客户端会读取 GitHub Releases 最新 release 下的 `latest.json`，下载并验证签名后的 Windows x64 安装包。

## Build Artifacts

本地打包产物位于：

```text
src-tauri/target/release/bundle/
```

CI x64 tag 构建产物位于：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/
```
