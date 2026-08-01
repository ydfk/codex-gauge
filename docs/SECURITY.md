# Security

Codex Gauge 只在用户本机查询与展示 Codex 用量。Windows 和 macOS 客户端均优先使用本机 `codex app-server`，失败时才在后端内存中读取 Codex OAuth 登录状态并请求用量接口。

## 安全边界

- 不保存完整 `auth.json`
- 不保存 `access_token`、`refresh_token`、Cookie 或认证请求头
- 不把认证字段传给 UI、配置文件或快照文件
- 不记录 app-server 原始响应、账号邮箱或完整唯一 ID
- 不抓取 ChatGPT 网页
- 不上传账号、用量、配置、快照或日志
- 不调用 `account/rateLimitResetCredit/consume`

## 本地数据

Windows 使用 `%APPDATA%\CodexGaugeNative\`：

- `config.json`：刷新、窗口、数据源与更新偏好
- `state.json`：脱敏后的最后快照和刷新状态
- `update.log`：只包含时间、操作、结果与泛化错误类别

macOS 使用 `~/Library/Application Support/Codex Gauge Native/`：

- `config.json`：菜单栏、刷新、数据源与更新偏好
- `snapshot.json`：脱敏后的最后成功快照

字段缺失、接口不支持、Codex 命令不存在或用户未登录时，客户端应显示未知或对应状态，不应崩溃，也不应把底层响应带入错误信息。

## 日志要求

日志不得包含 Token、Cookie、OAuth 凭据、Authorization 请求头、认证文件原文或服务端原始响应。更新与请求错误只记录 `network`、`invalid_auth`、`manifest`、`signature_invalid` 等稳定类别。

## 报告问题

提交安全问题时请只提供复现步骤、版本、平台和脱敏错误类别。不要在 Issue、截图或附件中上传认证文件和任何凭据。
