# 使用 GitHub Actions 发布 macOS ARM64 DMG

本文面向第一次配置 Apple Developer Program 的维护者，说明如何让 GitHub Actions 自动完成：

1. 构建仅支持 Apple Silicon 的 `arm64` 应用。
2. 使用 Developer ID Application 证书签名。
3. 制作带 Applications 快捷方式的 DMG。
4. 将 DMG 提交 Apple 公证并装订公证票据。
5. 使用 Sparkle Ed25519 私钥签署更新包并生成 `appcast.xml`。
6. 将 DMG 与 `appcast.xml` 发布到 GitHub Release。

最终用户打开 DMG，将 `Codex Gauge.app` 拖入 Applications 即可安装。应用内的“检查更新”由 Sparkle 完成，不需要 Mac App Store。

工作流运行在 `macos-15` runner，并主动选择镜像中最新的稳定 Xcode 26。不能依赖 `/Applications/Xcode.app`，因为该 runner 的默认 Xcode 可能仍是 16.x。

## 需要准备的内容

GitHub Actions 使用四个 Secrets 和四个 Variables：

| 类型 | 名称 | 内容 | 是否敏感 |
| --- | --- | --- | --- |
| Secret | `MACOS_CERTIFICATE_P12_BASE64` | Developer ID Application 证书与私钥导出的 `.p12`，再转换为 Base64 | 是 |
| Secret | `MACOS_CERTIFICATE_PASSWORD` | 导出 `.p12` 时设置的密码 | 是 |
| Secret | `APPLE_NOTARY_API_KEY_P8` | App Store Connect Team API Key 的 `.p8` 完整内容 | 是 |
| Secret | `SPARKLE_PRIVATE_KEY` | Sparkle 导出的 Ed25519 私钥 | 是 |
| Variable | `APPLE_TEAM_ID` | Apple Developer Team ID；当前工程团队为 `B6AA23TY3H` | 否 |
| Variable | `APPLE_NOTARY_KEY_ID` | App Store Connect Team API Key 的 Key ID | 否 |
| Variable | `APPLE_NOTARY_ISSUER_ID` | App Store Connect API 的 Issuer ID | 否 |
| Variable | `SPARKLE_PUBLIC_ED_KEY` | 与 Sparkle 私钥配对的 Ed25519 公钥 | 否 |

不要把 `.p12`、`.p8`、Sparkle 私钥、密码或包含这些内容的临时文件提交到仓库。

## 第一步：确认 Apple 账号已经可用

1. 登录 [Apple Developer Account](https://developer.apple.com/account/)。
2. 确认 Apple Developer Program 显示为有效状态。
3. 检查是否还有待接受的协议。
4. 在 Membership details 中找到 Team ID。
5. 登录 [App Store Connect](https://appstoreconnect.apple.com/)，确认可以进入“用户和访问”。

当前 Xcode 工程记录的 Team ID 是 `B6AA23TY3H`，Bundle ID 是 `com.ydfk.codex-gauge.native`。如果 Apple 后台显示的 Team ID 不同，应使用后台显示的真实值配置 `APPLE_TEAM_ID`，并在发布前确认这个 Bundle ID 属于该团队。

刚注册的账号可能需要等待 Apple 完成资格、协议或 App Store Connect API 权限的开通。证书、公证或 API 页面暂时不可用时，先检查账号状态，不要反复生成不同证书。

## 第二步：创建 Developer ID Application 证书

`Apple Development` 证书只能用于开发。通过 GitHub Release 在 Mac App Store 之外发布，需要 `Developer ID Application` 证书。DMG 分发不使用 `Developer ID Installer`；后者只用于 `.pkg` 安装包。

Apple 官方参考：

- [创建证书签名请求](https://developer.apple.com/help/account/certificates/create-a-certificate-signing-request)
- [创建 Developer ID 证书](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/)

### 2.1 创建证书签名请求

在自己的 Mac 上：

1. 打开“钥匙串访问”。
2. 在菜单中选择“钥匙串访问 → 证书助理 → 从证书颁发机构请求证书”。
3. 用户电子邮件地址填写 Apple Developer Program 使用的邮箱。
4. 常用名称填写便于识别的名称，例如 `Codex Gauge Developer ID`。
5. CA 电子邮件地址留空。
6. 选择“存储到磁盘”，保存 `.certSigningRequest` 文件。

必须在最终导出 `.p12` 的这台 Mac 上创建 CSR，因为对应私钥会保存在这台 Mac 的钥匙串中。

### 2.2 在 Apple Developer 后台生成证书

1. 打开 [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/certificates/list)。
2. 进入 Certificates，点击加号。
3. 在 Software 下选择 **Developer ID**。
4. 选择 **Developer ID Application**。
5. 上传刚生成的 `.certSigningRequest`。
6. 下载 `.cer` 文件并双击安装到钥匙串。

打开“钥匙串访问 → 登录 → 我的证书”。`Developer ID Application: ...` 左侧应可以展开，并在下面看到一把私钥。如果没有私钥，说明 CSR 来自另一台机器，当前证书无法用于 CI 签名。

也可以在终端检查：

```bash
security find-identity -v -p codesigning | grep "Developer ID Application"
```

### 2.3 导出 `.p12`

1. 在“我的证书”中选中 `Developer ID Application` 证书。
2. 确认证书下面的私钥也被包含。
3. 右键选择“导出”。
4. 格式选择 Personal Information Exchange (`.p12`)。
5. 设置一个强密码并记住它。
6. 将文件保存在仓库之外的安全位置。

假设导出的文件为 `/安全路径/DeveloperIDApplication.p12`，执行：

```bash
base64 < /安全路径/DeveloperIDApplication.p12 |
  gh secret set MACOS_CERTIFICATE_P12_BASE64 --repo ydfk/codex-gauge
```

再以交互方式设置 `.p12` 密码，避免密码出现在命令历史中：

```bash
gh secret set MACOS_CERTIFICATE_PASSWORD --repo ydfk/codex-gauge
```

终端会显示交互式 Secret 输入提示。粘贴密码并按回车即可。

## 第三步：创建 Apple 公证 API Key

工作流使用 `notarytool` 和 App Store Connect **Team API Key**。不要使用 Individual API Key；Apple 明确说明 Individual API Key 不能用于 `notarytool`。

Apple 官方参考：

- [App Store Connect API 入门](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api)
- [创建 App Store Connect API Key](https://developer.apple.com/documentation/appstoreconnectapi/creating-api-keys-for-app-store-connect-api)
- [自定义公证工作流](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)

### 3.1 首次申请 API 权限

1. 登录 [App Store Connect](https://appstoreconnect.apple.com/)。
2. 打开“用户和访问”。
3. 打开“集成”，选择 App Store Connect API。
4. 如果页面显示“请求访问”，由 Account Holder 接受条款并提交。
5. 等待 Apple 开通 API 权限后继续。

### 3.2 生成 Team API Key

1. 在“用户和访问 → 集成 → App Store Connect API”中选择 **Team Keys**。
2. 点击“生成 API Key”或加号。
3. 名称填写 `Codex Gauge GitHub Actions`。
4. Access 选择 **Developer**。它满足此工作流的公证用途，也避免授予不必要的管理权限。
5. 点击生成。
6. 记录页面显示的 **Issuer ID** 和该密钥的 **Key ID**。
7. 下载 `AuthKey_<KEY_ID>.p8`。

`.p8` 只能下载一次。立即将它备份到密码管理器或加密存储；如果丢失，只能撤销旧 Key 并重新创建。

### 3.3 写入 GitHub

将 `.p8` 完整内容写入 Secret：

```bash
gh secret set APPLE_NOTARY_API_KEY_P8 \
  --repo ydfk/codex-gauge \
  < /安全路径/AuthKey_REPLACE_WITH_KEY_ID.p8
```

将页面记录的 Key ID、Issuer ID 和 Team ID 写入 Variables：

```bash
gh variable set APPLE_NOTARY_KEY_ID \
  --repo ydfk/codex-gauge \
  --body "REPLACE_WITH_KEY_ID"

gh variable set APPLE_NOTARY_ISSUER_ID \
  --repo ydfk/codex-gauge \
  --body "REPLACE_WITH_ISSUER_ID"

gh variable set APPLE_TEAM_ID \
  --repo ydfk/codex-gauge \
  --body "B6AA23TY3H"
```

不要在占位符没有替换时执行命令。

## 第四步：生成 Sparkle 更新密钥

Apple Developer ID 签名证明应用来自你的开发者团队；Sparkle Ed25519 签名则防止应用内更新包被替换。两者用途不同，均不能把私钥提交到仓库。

Sparkle 官方参考：[发布 Sparkle 更新](https://sparkle-project.org/documentation/publishing/)。

### 4.1 下载 Sparkle 工具

在仓库根目录执行：

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  xcodebuild \
  -project native-macos/CodexGaugeNative.xcodeproj \
  -scheme CodexGaugeNative \
  -derivedDataPath native-macos/DerivedData \
  -resolvePackageDependencies
```

### 4.2 生成密钥对

只执行一次：

```bash
native-macos/DerivedData/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_keys \
  --account com.ydfk.codex-gauge.native
```

工具会：

- 把私钥保存在当前 Mac 的登录钥匙串中。
- 在终端输出 Base64 格式的公钥。

复制输出中的公钥，不要复制说明文字，然后设置 GitHub Variable：

```bash
gh variable set SPARKLE_PUBLIC_ED_KEY \
  --repo ydfk/codex-gauge \
  --body "REPLACE_WITH_SPARKLE_PUBLIC_KEY"
```

### 4.3 导出私钥到 GitHub Secret

先创建一个只用于本次操作的临时文件：

```bash
SPARKLE_KEY_FILE="$(mktemp -t codex-gauge-sparkle-key)"

native-macos/DerivedData/SourcePackages/artifacts/sparkle/Sparkle/bin/generate_keys \
  --account com.ydfk.codex-gauge.native \
  -x "$SPARKLE_KEY_FILE"

gh secret set SPARKLE_PRIVATE_KEY \
  --repo ydfk/codex-gauge \
  < "$SPARKLE_KEY_FILE"

rm -f "$SPARKLE_KEY_FILE"
```

第一次公开发布后不要随意重新生成 Sparkle 密钥。已经安装的客户端内置旧公钥，无法验证新私钥签署的更新。应把钥匙串中的 Sparkle 私钥额外备份到安全的离线位置。

## 第五步：检查 GitHub 配置

也可以在 GitHub 网页进入：

```text
Repository → Settings → Secrets and variables → Actions
```

Secrets 页应有：

```text
MACOS_CERTIFICATE_P12_BASE64
MACOS_CERTIFICATE_PASSWORD
APPLE_NOTARY_API_KEY_P8
SPARKLE_PRIVATE_KEY
```

Variables 页应有：

```text
APPLE_TEAM_ID
APPLE_NOTARY_KEY_ID
APPLE_NOTARY_ISSUER_ID
SPARKLE_PUBLIC_ED_KEY
```

使用 GitHub CLI 检查名称：

```bash
gh secret list --repo ydfk/codex-gauge
gh variable list --repo ydfk/codex-gauge
```

GitHub 不会显示 Secret 的内容，只显示名称和更新时间。完整配置应只存放在目标仓库的 Actions Secrets/Variables 中。GitHub 官方参考：[在 GitHub Actions 中使用 Secrets](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets)。

## 第六步：发布第一个版本

工作流接受 `v主版本.次版本.修订号` 形式的标签，例如 `v0.2.0`。版本号会自动写入 macOS 的 `CFBundleShortVersionString`，GitHub Actions 的递增运行编号会写入 `CFBundleVersion`。

先确保准备发布的提交已经推送，然后执行：

```bash
git status
git tag v0.2.0
git push origin v0.2.0
```

不要直接复制示例版本；应替换为实际要发布且尚未使用的版本号。

标签推送后，在 GitHub 的 Actions 页面打开 `release` 工作流。它会依次完成：

1. 校验标签并解析版本。
2. 构建 Windows x64。
3. 在 macOS runner 中创建临时钥匙串并导入 Developer ID Application 证书。
4. 仅构建 `arm64` 的 Release `.app`。
5. 验证 App 签名并制作 APFS/LZFSE 压缩 DMG。
6. 签名 DMG，使用 `notarytool` 最多等待 60 分钟完成 Apple 公证并装订票据。
7. 使用 Sparkle 私钥签署 DMG 并生成 `appcast.xml`；缺少有效更新签名时终止发布。
8. 汇总 Windows 与 macOS 产物并创建 GitHub Release。

macOS Release 应至少包含：

```text
Codex-Gauge-Native-<version>-macOS-arm64.dmg
appcast.xml
```

稳定版本的更新 feed 固定为：

```text
https://github.com/ydfk/codex-gauge/releases/latest/download/appcast.xml
```

预发布标签会创建 GitHub Prerelease，但 GitHub 的 `/releases/latest/` 不会指向 Prerelease。因此正式测试 Sparkle 更新时，应使用稳定版本标签。

## 第七步：验证发布结果

下载 Release 中的 DMG，然后验证签名、公证和架构：

```bash
xcrun stapler validate Codex-Gauge-Native-<version>-macOS-arm64.dmg

spctl --assess \
  --type open \
  --context context:primary-signature \
  --verbose=2 \
  Codex-Gauge-Native-<version>-macOS-arm64.dmg
```

双击 DMG，将 App 拖入 Applications。然后执行：

```bash
codesign --deep --strict --verify --verbose=2 \
  "/Applications/Codex Gauge.app"

spctl --assess --type execute --verbose=2 \
  "/Applications/Codex Gauge.app"

lipo -archs "/Applications/Codex Gauge.app/Contents/MacOS/CodexGaugeNative"
```

最后一个命令应只输出：

```text
arm64
```

检查线上 Sparkle feed：

```bash
curl -fL \
  https://github.com/ydfk/codex-gauge/releases/latest/download/appcast.xml
```

Sparkle 的完整验收需要两个稳定版本：先安装旧版本，再发布更高版本，然后在旧版本中执行“检查更新”，确认下载、校验、安装与重启全部成功。

## 常见问题

### 找不到 Developer ID Application identity

通常是 `.p12` 只包含证书、不包含私钥，或者上传的是 Apple Development/Apple Distribution 证书。回到钥匙串的“我的证书”，确认 Developer ID Application 可以展开并显示私钥，再重新导出。

### `notarytool` 提示未授权或找不到 Issuer

确认使用的是 Team API Key，而不是 Individual API Key；同时检查 `.p8`、Key ID 和 Issuer ID 是否属于同一把 Key。复制 ID 时不要包含前后空格。

### 新账号提示团队尚未配置公证

先确认会员状态有效、协议均已接受、Developer ID Application 证书有效、App Store Connect API 权限已批准。如果凭据验证和文件上传都成功，但 Apple 仍提示团队未配置公证，或第一次提交长时间没有结果，应保存 Actions 日志中的 submission ID，并联系 [Apple Developer Support](https://developer.apple.com/contact/)。

工作流最多等待公证 60 分钟。超时只表示 CI 停止等待，Apple 服务器可能仍在处理该 submission；先用日志中的 submission ID 查询状态，不要立即连续重试并创建大量重复提交。

可以在本机使用同一把 Team API Key 查询：

```bash
xcrun notarytool info REPLACE_WITH_SUBMISSION_ID \
  --key /安全路径/AuthKey_REPLACE_WITH_KEY_ID.p8 \
  --key-id REPLACE_WITH_KEY_ID \
  --issuer REPLACE_WITH_ISSUER_ID
```

### App 中“检查更新”不可用

检查 `SPARKLE_PUBLIC_ED_KEY` 是否为空或仍是占位值，并确认它与 `SPARKLE_PRIVATE_KEY` 来自同一对密钥。

### Sparkle 下载后提示签名无效

通常是重新生成了 Sparkle 私钥，或 GitHub 中的私钥、公钥不匹配。不要通过重新生成密钥掩盖问题；先从安全备份恢复原私钥。

### Developer ID 证书过期

创建新的 Developer ID Application 证书，重新导出 `.p12`，更新两个证书 Secrets。不要修改 Sparkle 密钥。已发布应用的 Sparkle 更新链路仍依赖原来的 Sparkle 公钥。

## 凭据安全边界

- GitHub Actions 只在临时 runner 钥匙串中导入 `.p12`，job 结束时删除钥匙串和临时文件。
- `.p8` 和 Sparkle 私钥只通过 Secrets 注入，不写入 artifact。
- `Config/Local.xcconfig` 由 CI 临时生成并在结束时删除。
- Developer ID 证书、App Store Connect API Key 或 Sparkle 私钥泄露后，应立即撤销或轮换对应凭据。
- Sparkle 密钥轮换会影响已安装客户端，不能像 Apple 公证 API Key 一样直接替换。
