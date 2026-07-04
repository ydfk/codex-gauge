# Codex Gauge

Codex Gauge 是一个轻量级 Windows 桌面浮窗，用于通过本机 Codex 登录状态查看 Codex 剩余用量。应用优先在 Rust 后端内存中读取 Codex OAuth 登录状态的必要字段，请求 ChatGPT wham 用量接口；失败时回退本机 `codex app-server`。它不保存 Token，也不上传任何用户数据。

## Features

- Tauri v2 + Svelte + TypeScript + Rust
- Windows 透明无边框浮窗，默认约 `220x92`
- 系统托盘常驻，支持打开、隐藏、刷新、设置和退出
- 优先使用本机 `.codex/auth.json` 登录状态查询 wham 用量接口
- `auth-json` 不可用时回退 stdio JSON-RPC `codex app-server`
- 显示 5 小时窗口、Weekly 窗口和可用重置次数
- 本地 JSON 保存配置、状态和最多 90 天用量历史
- GitHub Releases 在线检查更新，支持手动安装
- Windows x64 GitHub Actions 打包发布

## Security

Codex Gauge 遵守这些边界：

- 只在 Rust 后端内存中读取 `CODEX_HOME/auth.json` 或 `~/.codex/auth.json` 的必要字段
- 不保存 `access_token`、`refresh_token`、Cookie 或任何敏感凭据
- 不把认证字段传给前端、状态文件或历史文件
- 不抓取 ChatGPT 网页
- 不上传账号、用量、配置或历史数据
- 不输出 app-server 原始响应、账号邮箱明文或 Token
- 不调用消耗重置次数的接口

更多说明见 [SECURITY.md](docs/SECURITY.md)。

## Requirements

- Windows 10/11 x64
- Node.js 26+
- pnpm 11+
- Rust stable
- 已登录的 Codex CLI 或 Codex Desktop 登录状态

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
pnpm release:check    # 发布流水线轻量检查
pnpm rust:check       # Rust fmt/check/test
pnpm rust:fmt         # 格式化 Rust
pnpm tauri:dev        # 启动桌面应用
pnpm tauri:build      # 本地 Windows 打包
pnpm diagnose:codex   # 脱敏诊断 Codex CLI/app-server
pnpm version:sync v0.1.0
```

## Troubleshooting Unknown Data

如果界面一直显示“未知”，通常是下面几类原因：

1. `CODEX_HOME/auth.json` 或 `~/.codex/auth.json` 不存在，或没有可用登录状态。
2. Codex 登录凭据过期，wham 接口返回 401/403，需要重新登录 Codex。
3. 网络请求失败，或 wham 接口结构变化，解析器只能降级显示“未知”。
4. `codex app-server` 回退不可用，例如 `codex` 命令不存在或被 Windows 拒绝访问。

先运行脱敏诊断：

```bash
pnpm diagnose:codex
```

诊断脚本不会输出认证信息、app-server 原始响应或请求头。重点看：

- `codex --version` 是否 `[OK]`
- `initialize / initialized` 是否 `[OK]`
- `account/rateLimits/read` 是否 `[OK]`
- 如果 app-server 不可用，应用仍会优先尝试 AuthJson Provider

如果 wham 查询返回“Codex 凭据无效”，请重新登录 Codex。若 `codex --version` 是 `permission_denied`，请检查 Codex 安装来源、WindowsApps 权限，或在设置里把 Codex command 改为可执行的真实路径。

## Local Data

Windows 默认路径：

- `%APPDATA%\CodexGauge\config.json`
- `%APPDATA%\CodexGauge\state.json`
- `%APPDATA%\CodexGauge\usage-history.json`

这些文件只保存配置、脱敏状态和本地统计，不包含 OAuth Token、Cookie、请求头或 app-server 原始响应。

## Release

GitHub Actions 在 SemVer tag 上打包 Windows x64：

```bash
git tag v0.1.0
git push origin v0.1.0
```

如果只需要生成安装包，可以直接推送 tag。若需要应用内“检查更新/安装更新”生成可用的 `latest.json` 和签名文件，需要在 GitHub 仓库中配置：

- Repository variable: `TAURI_UPDATER_PUBKEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，如果私钥设置了密码

`TAURI_UPDATER_PUBKEY` 推荐配置在 Repository variables；如果误配置在 Repository secrets，CI 也会读取同名 secret 作为兼容兜底。

生成 updater 密钥：

```bash
pnpm tauri signer generate --write-keys updater.key
```

`src-tauri/tauri.conf.json` 默认使用：

```text
https://github.com/liyuhang/codex-gauge/releases/latest/download/latest.json
```

如果你的 GitHub 仓库不是 `liyuhang/codex-gauge`，CI 发布时会自动把 endpoint 改为当前 `${{ github.repository }}`。没有配置 updater 公钥或签名私钥时，CI 会跳过 `latest.json` 和签名文件，但仍会发布 Windows x64 安装包。

## Update Flow

应用设置页提供“检查更新”和“安装更新”。配置 updater 签名后，客户端会读取 GitHub Releases 最新 release 下的 `latest.json`，下载并验证签名后的 Windows x64 安装包。

## Build Artifacts

本地打包产物位于：

```text
src-tauri/target/release/bundle/
```

CI x64 tag 构建产物位于：

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/
```
