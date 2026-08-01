# Release

Windows 与 macOS 使用独立的原生构建和更新链路。版本遵循 SemVer，两套更新清单不可互换。

## Windows x64

本地检查与构建：

```powershell
.\native-windows\scripts\check.ps1
.\native-windows\scripts\build.ps1
```

推送 `v*.*.*` 标签会触发 `.github/workflows/release.yml`：

```powershell
git tag v0.2.0
git push origin v0.2.0
```

GitHub 仓库需要配置：

| 类型 | 名称 | 用途 |
| --- | --- | --- |
| Secret | `NATIVE_WINDOWS_SIGNING_PRIVATE_KEY` | 完整 Minisign 私钥内容 |
| Secret | `NATIVE_WINDOWS_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；无密码时可留空 |
| Variable 或 Secret | `NATIVE_WINDOWS_UPDATER_PUBKEY` | 编译进客户端的 Minisign 公钥 |

工作流构建 Windows x64 便携 EXE，使用仓库内的纯 Rust 签名工具生成 `.sig`，并发布：

- `Codex.Gauge.Native_<version>_x64.portable.exe`
- 对应 `.sig`
- `latest.json`

GitHub 最新正式 Release 保存更新清单。客户端通过 `releases/latest/download/latest.json` 检查更新，并只安装签名验证通过的 `windows-x86_64` 资产。

## macOS

macOS 使用 Sparkle appcast。首次发布前需要：

1. 在 `native-macos` 中解析 Sparkle 依赖。
2. 使用 Sparkle `generate_keys` 生成 Ed25519 密钥。
3. 从 `Config/Local.xcconfig.example` 创建未提交的 `Config/Local.xcconfig` 并写入公钥。
4. 配置 Developer ID Application 签名与 Apple 公证凭据。
5. 执行 `native-macos/scripts/release.sh`。

脚本生成签名 zip 与 `appcast.xml`，产物位于 `native-macos/build/update/`。详细步骤见 [macOS 发布说明](../native-macos/docs/RELEASE.md)。

## 发布验收

- Windows：从旧版本执行检查、下载、签名校验、替换和重启
- macOS：验证 `codesign`、`spctl`、公证状态，并从旧版本完成 Sparkle 更新
- Release 不包含私钥、认证文件、Token、Cookie、配置或本地日志
- 清单中的版本、平台和下载 URL 与实际资产一致
