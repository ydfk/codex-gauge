# Security

Codex Gauge 优先通过本机 Codex OAuth 登录状态查询 wham 用量接口，失败时回退本机 `codex app-server`。认证字段只在 Rust 后端内存中短暂使用，不保存、不传给前端、不写入日志。

## 不做的事情

- 不保存完整 `auth.json` 内容
- 不保存 `access_token`、`refresh_token`、OAuth Cookie 或 ChatGPT Cookie
- 不把认证字段传给前端、状态文件或历史文件
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
- Authorization 请求头
