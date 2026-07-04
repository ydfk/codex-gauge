<p align="center">
  <img src="src-tauri/icons/icon.png" width="96" alt="Codex Gauge icon" />
</p>

<h1 align="center">Codex Gauge</h1>

<p align="center">
  轻量桌面浮窗，用于在本机查看 Codex 5h / 7d 用量、重置时间和可用重置次数。
</p>

<p align="center">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square" />
  <img alt="Svelte" src="https://img.shields.io/badge/Svelte-5-ff3e00?style=flat-square&logo=svelte&logoColor=white" />
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?style=flat-square&logo=rust&logoColor=white" />
  <img alt="Windows x64" src="https://img.shields.io/badge/Windows-x64-0078d4?style=flat-square&logo=windows&logoColor=white" />
</p>

## Overview

Codex Gauge 是一个本机桌面监控工具，使用 Tauri v2、Svelte、TypeScript 和 Rust 构建。它提供一个桌面浮窗和一个顶部迷你状态条，用于快速查看 Codex 当前用量。

默认数据来源是本机 `codex app-server`；当 app-server 不可用时，可以回退到本机 Codex 登录状态的 AuthJson Provider。重置次数目前只能通过 AuthJson API 查询。

> [!IMPORTANT]
> Codex Gauge 只面向个人本机使用。它不会上传数据，不做远程代理，不保存 Token，也不会抓取 ChatGPT 网页。

## Features

- 深色 Liquid Glass 风格桌面浮窗
- 顶部迷你状态条，展示 5h、7d 剩余量和重置次数
- 双击浮窗进入详情页，再次双击返回
- 系统托盘菜单：打开/隐藏浮窗、固定/取消固定、刷新、打开设置、退出
- 5h 和 7d 用量窗口解析
- 可用重置次数读取和 reset credit 详情展示
- 本地自动刷新和手动刷新
- 本地 JSON 配置、状态和历史记录
- 可选开机启动、OLED 防烧屏微位移、顶部状态条开关
- GitHub Releases 手动检查更新
- GitHub Actions Windows x64 打包发布

## Screens

当前应用包含两个常驻窗口：

- **桌面浮窗**：显示 Codex、5h/7d 进度条、重置时间和重置次数
- **顶部状态条**：默认位于桌面顶部中心，只显示关键状态

两个窗口都支持右键菜单。桌面浮窗默认不置顶，顶部状态条默认置顶。

## Requirements

- Windows 10/11 x64
- Node.js 26+
- pnpm 11+
- Rust stable
- Codex CLI 或 Codex Desktop 的本机登录状态

> [!NOTE]
> 当前 CI 只打包 Windows x64。Tauri 项目结构保留跨平台基础，但 macOS 尚未作为发布目标验证。

## Getting Started

安装依赖：

```bash
pnpm install
```

启动桌面应用：

```bash
pnpm dev:desktop
```

本地打包：

```bash
pnpm tauri:build
```

## Data Sources

Codex Gauge 当前支持三类数据来源。

| Source | 用途 | 说明 |
| --- | --- | --- |
| `app-server` | 主用量 | 默认优先，通过 `codex app-server` stdio JSON-RPC 查询 |
| `auth-json` | 主用量回退、重置次数 | 读取本机 Codex OAuth 登录状态中的必要字段，仅在 Rust 后端内存中使用 |
| `session-log` | Token/诊断兜底 | 不用于推断 5h/7d 剩余额度 |

设置页可以调整主用量优先方式：

- `app-server`：默认方式，没有可用 app-server 时回退 API
- `api`：优先使用 AuthJson Provider

重置次数固定使用 AuthJson API，因为 app-server 当前不提供对应数据。

## Privacy And Safety

Codex Gauge 遵守以下边界：

- 不保存 `access_token`、`refresh_token`、Cookie 或认证请求头
- 不把认证字段传给前端
- 不把认证字段写入 `config.json`、`state.json` 或 `usage-history.json`
- 不输出 app-server 原始响应
- 不输出账号邮箱明文、Token、Cookie 或完整唯一 ID
- 不抓取 ChatGPT 网页
- 不上传账号、用量、配置或历史数据
- 不调用消耗重置次数的接口

更多细节见 [docs/SECURITY.md](docs/SECURITY.md)。

## Local Data

Windows 默认写入目录：

```text
%APPDATA%\CodexGauge\
```

文件说明：

| File | 内容 |
| --- | --- |
| `config.json` | 本地设置、窗口偏好、刷新间隔 |
| `state.json` | 脱敏后的最后快照和本地统计 |
| `usage-history.json` | 最多 90 天的本地聚合历史 |

这些文件不包含 OAuth Token、Cookie、请求头或完整认证响应。

## Scripts

| Command | Description |
| --- | --- |
| `pnpm dev` | 启动 Vite 前端 |
| `pnpm dev:desktop` | 启动 Tauri 桌面应用 |
| `pnpm build` | 前端类型检查与构建 |
| `pnpm check` | 完整本地检查：前端构建、Rust fmt/check/test |
| `pnpm release:check` | 发布流水线轻量检查 |
| `pnpm rust:check` | Rust fmt/check/test |
| `pnpm rust:fmt` | 格式化 Rust |
| `pnpm tauri:build` | 本地打包 |
| `pnpm diagnose:codex` | 脱敏诊断 Codex CLI/app-server |
| `pnpm diagnose:credits` | 脱敏诊断重置次数 API |
| `pnpm version:sync v0.1.0` | 同步 package 与 Tauri 版本 |

## Troubleshooting

### 界面显示“未知”

常见原因：

- `codex app-server` 不可用，或 `codex` 命令不在 `PATH`
- Codex 登录状态不存在或已过期
- AuthJson Provider 请求返回 401/403
- wham 接口结构变化，字段无法识别
- 网络请求失败

先运行：

```bash
pnpm diagnose:codex
pnpm diagnose:credits
```

诊断脚本会脱敏输出，不会打印 Token、Cookie、请求头或完整认证文件。

### Codex Desktop 没有 `codex` 命令

如果没有可用的 `codex app-server`，可以在设置页把主用量优先方式改为 `api`。这会使用本机 Codex 登录状态查询用量接口。若 API 返回凭据无效，请重新登录 Codex。

### 重置次数一直未知

重置次数只通过 AuthJson API 查询。请确认：

- 本机存在 Codex 登录状态
- 登录状态未过期
- `pnpm diagnose:credits` 能读取到 `available_count`

## Release

GitHub Actions 会在 SemVer tag 上打包 Windows x64：

```bash
git tag v0.1.0
git push origin v0.1.0
```

也可以在 Actions 页面手动运行 `release` workflow，并输入 tag，例如：

```text
v0.1.0
```

### Updater configuration

如果只需要发布安装包，不需要配置 updater 变量。CI 会上传 `.msi` 和 `.exe`。

如果需要应用内“检查更新 / 安装更新”，需要配置：

| Type | Name | Required |
| --- | --- | --- |
| Repository variable | `TAURI_UPDATER_PUBKEY` | Yes |
| Repository secret | `TAURI_SIGNING_PRIVATE_KEY` | Yes |
| Repository secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Only if the key has a password |

`TAURI_UPDATER_PUBKEY` 推荐放在 Repository variables。若放在 Repository secrets，当前 workflow 也会读取同名 secret 作为兜底。

应用内默认更新地址为：

```text
https://github.com/ydfk/codex-gauge/releases/latest/download/latest.json
```

也可以在设置页修改为自己的 GitHub Release `latest.json` 地址。

签名配置完整时，CI 会根据 updater `.zip` 和 `.sig` 签名文件生成 `latest.json`，然后一起上传。缺少这些文件会让发布流程失败，避免 Release 看起来成功但应用内更新不可用。

生成 updater 密钥：

```bash
pnpm tauri signer generate --write-keys updater.key
```

不要把私钥提交到仓库。更多发布说明见 [docs/RELEASE.md](docs/RELEASE.md)。

## Project Structure

```text
.
├─ src/                  # Svelte 前端
├─ src-tauri/            # Rust/Tauri 后端
├─ scripts/              # 版本同步和脱敏诊断脚本
├─ docs/                 # 安全、发布和设计文档
└─ .github/workflows/    # Windows x64 release workflow
```
