# 原生 macOS 发布

macOS 客户端通过 GitHub Actions 发布仅支持 Apple Silicon 的 ARM64 DMG，并使用 Sparkle `appcast.xml` 提供应用内自动更新。

## 发布安全链路

- Developer ID Application 签名用于证明应用来自已验证的 Apple Developer 团队。
- Apple 公证扫描发布内容，并把可供 Gatekeeper 验证的票据装订到 DMG。
- Sparkle Ed25519 签名保护应用内更新包，防止更新资源被替换。

这三项用途不同。Sparkle 不替代 Apple 签名与公证，Apple 签名也不替代 Sparkle 更新签名。

## GitHub Actions 发布

推送 `v*.*.*` 标签后，[release 工作流](../../.github/workflows/release.yml)会：

1. 使用标签设置应用版本，使用 Actions 运行编号设置构建号。
2. 仅构建 `arm64` Release 归档。
3. 创建带 Applications 快捷方式的 APFS/LZFSE DMG。
4. 签名、公证并装订 DMG。
5. 使用 Sparkle 私钥签署 DMG 并生成 `appcast.xml`。
6. 将 macOS 与 Windows 产物发布到同一个 GitHub Release。

第一次配置 Apple Developer 证书、公证 API Key、Sparkle 密钥和 GitHub Secrets/Variables 时，请完整执行[GitHub Actions macOS 发布配置指南](GITHUB_ACTIONS_RELEASE.md)。

## 本地执行发布脚本

CI 与本地共用 `scripts/release.sh`。脚本要求：

- 本机钥匙串存在 Developer ID Application 证书及私钥。
- `Config/Local.xcconfig` 包含 Sparkle 公钥。
- 已设置 Apple Team ID、公证 Team API Key 路径、Key ID 和 Issuer ID。
- Sparkle 私钥存在于钥匙串，或通过 `SPARKLE_PRIVATE_KEY` 环境变量传入。

脚本生成：

```text
build/update/Codex-Gauge-Native-<version>-macOS-arm64.dmg
build/update/appcast.xml
```

稳定版本的 feed 固定为：

```text
https://github.com/ydfk/codex-gauge/releases/latest/download/appcast.xml
```

## 发布验收

发布成功后仍需完成真实安装和更新验收：

```bash
xcrun stapler validate Codex-Gauge-Native-<version>-macOS-arm64.dmg

spctl --assess \
  --type open \
  --context context:primary-signature \
  --verbose=2 \
  Codex-Gauge-Native-<version>-macOS-arm64.dmg
```

还需要从旧的稳定版本实际执行“检查更新 → 下载 → 校验 → 安装 → 重启”。本地无签名构建成功不能证明 Developer ID、公证或线上 Sparkle 链路可用。
