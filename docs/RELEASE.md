# Release

## 版本

版本使用 SemVer：

- `0.1.0`：第一个可用版本
- `0.2.0`：Token 统计增强
- `0.3.0`：自动升级完善
- `1.0.0`：稳定版

## Windows 构建

```bash
pnpm install
pnpm build
pnpm tauri:build
```

产物位于 `src-tauri/target/release/bundle`。

## GitHub Actions 发布

推送 SemVer tag 后会触发 `.github/workflows/release.yml`：

```bash
git tag v0.1.0
git push origin v0.1.0
```

当前 CI 只生成 Windows x64 产物，并上传到 GitHub Release。

## Updater

应用通过 GitHub Release 的 `latest.json` 检测最新版本。只生成安装包时不需要 updater 配置；需要应用内更新时，需要在 GitHub 仓库配置：

- Repository variable: `TAURI_UPDATER_PUBKEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY`
- Repository secret: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，如果私钥有密码

`TAURI_UPDATER_PUBKEY` 推荐放在 Repository variables。若放在 Repository secrets，当前 CI 也会读取同名 secret 作为兜底。

正式发布前需要：

1. 生成 Tauri updater 签名密钥。
2. 将 public key 写入 `plugins.updater.pubkey`。
3. 将 endpoint 改为真实 GitHub Releases `latest.json` 地址，或使用 CI 自动替换为当前仓库。
4. 在 CI 中将 `bundle.createUpdaterArtifacts` 改为 `true`。
5. 在 CI 中设置 `TAURI_SIGNING_PRIVATE_KEY`。
6. 使用 CI 上传安装包、签名和 `latest.json`。

不要把私钥提交到仓库。

本地默认 `createUpdaterArtifacts = false`，用于避免没有签名私钥时普通构建失败。CI 也会在缺少 `TAURI_UPDATER_PUBKEY` 或 `TAURI_SIGNING_PRIVATE_KEY` 时跳过 `latest.json` 和签名文件，只发布 Windows x64 安装包。
