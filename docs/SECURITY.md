# Security

Codex Gauge 只通过本机 `codex app-server` 获取状态，不直接读取 Codex 的认证文件。

## 不做的事情

- 不读取 `~/.codex/auth.json`
- 不保存 `access_token`、`refresh_token`、OAuth Cookie 或 ChatGPT Cookie
- 不抓取 ChatGPT 网页
- 不上传账号、用量、配置或历史数据
- 不记录 app-server 原始响应
- 不调用 `account/rateLimitResetCredit/consume`

## 本地存储

应用只写入 `%APPDATA%\CodexGauge` 下的 JSON 文件：

- `config.json`：窗口、刷新间隔、显示偏好
- `state.json`：脱敏后的最后快照和本地重置统计
- `usage-history.json`：最多 90 天的本地聚合快照

字段缺失、接口不可用、Codex CLI 不存在或用户未登录时，应用应显示“未知”或对应状态，不应崩溃。

## 日志原则

日志只能记录错误类别和脱敏状态，不得包含：

- app-server 原始响应
- 邮箱明文
- Token
- Cookie
- OAuth 凭据
