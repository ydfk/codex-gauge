# 原生 macOS 发布

## 为什么使用独立 appcast

Tauri updater 的 `latest.json` 与 Sparkle 的 appcast 格式不兼容。原生版使用 `appcast.xml`，避免把更新安装、权限提升、原子替换和重启逻辑重新实现一遍。

Sparkle 会校验更新包的 Ed25519 签名；正式分发仍应同时使用 Apple Developer ID 签名与公证。两者分别防止更新源被篡改和应用身份被替换。

## 一次性配置

先解析依赖：

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcodebuild \
  -project CodexGaugeNative.xcodeproj \
  -scheme CodexGaugeNative \
  -resolvePackageDependencies
```

生成 Sparkle 密钥：

```bash
DerivedData/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_keys
```

工具会把私钥保存在钥匙串并输出公钥。创建未提交配置：

```bash
cp Config/Local.xcconfig.example Config/Local.xcconfig
```

将 `SPARKLE_PUBLIC_ED_KEY` 替换为真实公钥。应用检测到占位值时会禁用检查更新按钮，避免把配置问题伪装成网络失败。

## 构建签名归档

在 Xcode 的 Signing & Capabilities 中选择 Developer ID 团队，确认：

- Bundle ID 为 `com.ydfk.codex-gauge.native`。
- Hardened Runtime 已启用。
- Release 构建使用 Developer ID Application 证书。
- 公证凭据只存在本机钥匙串或 CI Secret。

然后执行：

```bash
./scripts/release.sh
```

脚本会：

1. 创建 Release `.xcarchive`。
2. 把 `.app` 压缩为 Sparkle 可安装的 zip。
3. 使用 Sparkle `generate_appcast` 生成签名与 `appcast.xml`。
4. 把产物放到 `build/update/`。

## 发布资源

上传以下文件到同一个 GitHub Release：

- `Codex-Gauge-Native-<version>-macOS.zip`
- `appcast.xml`

应用内 feed 固定为：

```text
https://github.com/ydfk/codex-gauge/releases/latest/download/appcast.xml
```

因此 `appcast.xml` 中 enclosure 的下载地址必须指向公开 HTTPS 资源。发布后至少验证：

```bash
curl -fL https://github.com/ydfk/codex-gauge/releases/latest/download/appcast.xml
codesign --deep --strict --verify "Codex Gauge.app"
spctl --assess --type execute --verbose "Codex Gauge.app"
```

还需要从一个旧的已签名版本实际执行“检查更新 → 下载 → 安装 → 重启”。本地无签名构建成功不能证明签名迁移、公证或线上更新链路可用。

