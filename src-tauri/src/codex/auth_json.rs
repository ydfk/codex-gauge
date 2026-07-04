use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use serde_json::Value;

use super::{
    CodexUsageSnapshot, ResetCreditItem, SnapshotSource, SnapshotStatus, UsageCredits, UsageWindow,
};

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CREDITS_URL: &str = "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthJsonError {
    NotLoggedIn,
    InvalidAuth,
    RequestFailed,
}

impl AuthJsonError {
    pub fn status(self) -> SnapshotStatus {
        match self {
            Self::NotLoggedIn => SnapshotStatus::NotLoggedIn,
            Self::InvalidAuth => SnapshotStatus::InvalidAuth,
            Self::RequestFailed => SnapshotStatus::RequestFailed,
        }
    }
}

impl std::fmt::Display for AuthJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NotLoggedIn => "未检测到 Codex 登录状态",
            Self::InvalidAuth => "Codex 凭据无效",
            Self::RequestFailed => "Codex 用量查询失败",
        };
        f.write_str(message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthInfo {
    access_token: String,
    account_id: Option<String>,
    plan_type: Option<String>,
}

pub fn read_auth_json_snapshot() -> Result<CodexUsageSnapshot, AuthJsonError> {
    let path = resolve_auth_path();
    let auth = read_auth_info(&path)?;
    fetch_snapshot(&auth)
}

pub fn read_auth_json_credits() -> Result<Option<UsageCredits>, AuthJsonError> {
    let path = resolve_auth_path();
    let auth = read_auth_info(&path)?;
    fetch_credits(&auth)
}

fn resolve_auth_path() -> PathBuf {
    let codex_home = env::var_os("CODEX_HOME").map(PathBuf::from);
    let home = dirs::home_dir();
    resolve_auth_path_from(codex_home, home)
}

fn resolve_auth_path_from(codex_home: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    if let Some(path) = codex_home {
        return path.join("auth.json");
    }

    home.unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("auth.json")
}

fn read_auth_info(path: &Path) -> Result<AuthInfo, AuthJsonError> {
    let content = fs::read_to_string(path).map_err(|_| AuthJsonError::NotLoggedIn)?;
    parse_auth_json(&content)
}

fn parse_auth_json(content: &str) -> Result<AuthInfo, AuthJsonError> {
    let value: Value = serde_json::from_str(content).map_err(|_| AuthJsonError::NotLoggedIn)?;
    let Some(access_token) = find_string_by_keys(&value, &["access_token", "accessToken"]) else {
        return Err(AuthJsonError::NotLoggedIn);
    };

    if access_token.trim().is_empty() {
        return Err(AuthJsonError::NotLoggedIn);
    }

    Ok(AuthInfo {
        access_token,
        account_id: find_string_by_keys(
            &value,
            &[
                "account_id",
                "accountId",
                "chatgpt_account_id",
                "chatgptAccountId",
            ],
        ),
        plan_type: find_string_by_keys(&value, &["plan_type", "planType", "plan"]),
    })
}

fn fetch_snapshot(auth: &AuthInfo) -> Result<CodexUsageSnapshot, AuthJsonError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| AuthJsonError::RequestFailed)?;
    let headers = build_headers(auth)?;
    let usage = request_json(&client, USAGE_URL, &headers);
    let credits = request_json(&client, CREDITS_URL, &headers);

    if matches!(usage, Err(AuthJsonError::InvalidAuth))
        || matches!(credits, Err(AuthJsonError::InvalidAuth))
    {
        return Err(AuthJsonError::InvalidAuth);
    }

    let credits = credits.ok().as_ref().and_then(parse_reset_credits);
    let Ok(usage) = usage else {
        let mut snapshot = super::snapshot::empty_snapshot(
            SnapshotSource::AuthJson,
            SnapshotStatus::RequestFailed,
        );
        snapshot.credits = credits;
        if snapshot.plan_type.is_none() {
            snapshot.plan_type = auth.plan_type.clone();
        }
        return Ok(snapshot);
    };

    let mut snapshot = parse_wham_usage(&usage);
    snapshot.credits = credits.or(snapshot.credits);
    if snapshot.plan_type.is_none() {
        snapshot.plan_type = auth.plan_type.clone();
    }

    Ok(snapshot)
}

fn fetch_credits(auth: &AuthInfo) -> Result<Option<UsageCredits>, AuthJsonError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| AuthJsonError::RequestFailed)?;
    let headers = build_headers(auth)?;
    request_json(&client, CREDITS_URL, &headers).map(|value| parse_reset_credits(&value))
}

fn build_headers(auth: &AuthInfo) -> Result<HeaderMap, AuthJsonError> {
    let mut headers = HeaderMap::new();
    let auth_value = format!("Bearer {}", auth.access_token);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_value).map_err(|_| AuthJsonError::InvalidAuth)?,
    );
    headers.insert(
        HeaderName::from_static("openai-beta"),
        HeaderValue::from_static("codex-1"),
    );
    headers.insert(
        HeaderName::from_static("originator"),
        HeaderValue::from_static("Codex Desktop"),
    );
    if let Some(account_id) = auth.account_id.as_deref() {
        if let Ok(value) = HeaderValue::from_str(account_id) {
            headers.insert("ChatGPT-Account-ID", value);
        }
    }
    Ok(headers)
}

fn request_json(client: &Client, url: &str, headers: &HeaderMap) -> Result<Value, AuthJsonError> {
    let response = client
        .get(url)
        .headers(headers.clone())
        .send()
        .map_err(|_| AuthJsonError::RequestFailed)?;
    let status = response.status();

    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return Err(AuthJsonError::InvalidAuth);
    }
    if !status.is_success() {
        return Err(AuthJsonError::RequestFailed);
    }

    response
        .json::<Value>()
        .map_err(|_| AuthJsonError::RequestFailed)
}

pub fn parse_wham_usage(value: &Value) -> CodexUsageSnapshot {
    let root = value.get("result").unwrap_or(value);
    let mut snapshot =
        super::snapshot::empty_snapshot(SnapshotSource::AuthJson, SnapshotStatus::Ok);
    let mut windows = Vec::new();

    collect_wham_windows(root, &mut windows);
    for window in windows {
        match window.name.as_str() {
            "5h" if snapshot.primary_window.is_none() => snapshot.primary_window = Some(window),
            "weekly" if snapshot.secondary_window.is_none() => {
                snapshot.secondary_window = Some(window)
            }
            _ => {}
        }
    }

    snapshot.plan_type = find_string_by_keys(root, &["plan_type", "planType", "plan"]);
    snapshot.rate_limit_reached_type = find_string_by_keys(root, &["rateLimitReachedType"]);
    snapshot.credits = parse_reset_credits(root);
    snapshot
}

pub fn parse_reset_credits(value: &Value) -> Option<UsageCredits> {
    let root = value.get("result").unwrap_or(value);
    let count_root = root.get("rateLimitResetCredits").unwrap_or(root);
    let mut items = collect_reset_credit_items(root);
    if !std::ptr::eq(root, count_root) {
        items.extend(collect_reset_credit_items(count_root));
    }

    let credits = UsageCredits {
        remaining: first_number_i64(count_root, &["remaining", "remainingCount"]),
        available_count: first_number_i64(
            count_root,
            &[
                "available_count",
                "availableCount",
                "available",
                "availableCredits",
            ],
        ),
        reset_credits: first_number_i64(
            count_root,
            &[
                "availableCount",
                "available_count",
                "resetCredits",
                "reset_credits",
            ],
        ),
        reset_at: first_timestamp(
            count_root,
            &["resetAt", "reset_at", "resetsAt", "resets_at"],
        ),
        items,
    };

    if credits.remaining.is_none()
        && credits.available_count.is_none()
        && credits.reset_credits.is_none()
        && credits.reset_at.is_none()
        && credits.items.is_empty()
    {
        None
    } else {
        Some(credits)
    }
}

fn collect_reset_credit_items(value: &Value) -> Vec<ResetCreditItem> {
    let Some(items) = find_credit_array(value) else {
        return Vec::new();
    };

    items.iter().filter_map(parse_reset_credit_item).collect()
}

fn find_credit_array(value: &Value) -> Option<&Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }

    let object = value.as_object()?;
    for key in [
        "credits",
        "items",
        "data",
        "resetCredits",
        "reset_credits",
        "rateLimitResetCredits",
    ] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            return Some(items);
        }
    }

    None
}

fn parse_reset_credit_item(value: &Value) -> Option<ResetCreditItem> {
    let item = ResetCreditItem {
        status: first_string_value(value, &["status", "state"]),
        title: first_string_value(value, &["title", "displayTitle", "display_title", "name"]),
        granted_at: first_local_time(
            value,
            &["granted_at", "grantedAt", "created_at", "createdAt"],
        ),
        expires_at: first_local_time(
            value,
            &["expires_at", "expiresAt", "expiration_at", "expirationAt"],
        ),
    };

    if item.status.is_none()
        && item.title.is_none()
        && item.granted_at.is_none()
        && item.expires_at.is_none()
    {
        None
    } else {
        Some(item)
    }
}

fn collect_wham_windows(value: &Value, output: &mut Vec<UsageWindow>) {
    if let Some(window) = parse_wham_window(value) {
        output.push(window);
        return;
    }

    if let Some(values) = value.as_object() {
        for child in values.values() {
            collect_wham_windows(child, output);
        }
        return;
    }

    if let Some(values) = value.as_array() {
        for child in values {
            collect_wham_windows(child, output);
        }
    }
}

fn parse_wham_window(value: &Value) -> Option<UsageWindow> {
    let window_duration_seconds = first_number_i64(
        value,
        &[
            "limit_window_seconds",
            "limitWindowSeconds",
            "windowDurationSeconds",
            "window_duration_seconds",
        ],
    );
    let name = match window_duration_seconds {
        Some(18_000) => "5h",
        Some(604_800) => "weekly",
        _ => return None,
    };
    let raw_used_percent = first_number_f64(
        value,
        &[
            "usedPercent",
            "used_percent",
            "usagePercent",
            "usage_percent",
            "currentUsagePercent",
        ],
    )
    .map(normalize_percent);
    let raw_remaining_percent = first_number_f64(
        value,
        &[
            "remainingPercent",
            "remaining_percent",
            "remainingPercentage",
            "remaining_percentage",
        ],
    )
    .map(normalize_percent);
    let used_percent =
        raw_used_percent.or_else(|| raw_remaining_percent.map(|remaining| 100.0 - remaining));
    let remaining_percent =
        raw_remaining_percent.or_else(|| used_percent.map(|used| (100.0 - used).clamp(0.0, 100.0)));

    Some(UsageWindow {
        name: name.to_string(),
        used_percent,
        remaining_percent,
        reset_at: first_timestamp(
            value,
            &[
                "resetAt",
                "reset_at",
                "resetsAt",
                "resets_at",
                "expiresAt",
                "expires_at",
            ],
        ),
        window_duration_seconds,
    })
}

fn find_string_by_keys(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(object) = value.as_object() {
        for key in keys {
            if let Some(text) = object.get(*key).and_then(Value::as_str) {
                return Some(text.to_string());
            }
        }
        for child in object.values() {
            if let Some(text) = find_string_by_keys(child, keys) {
                return Some(text);
            }
        }
    }

    if let Some(values) = value.as_array() {
        for child in values {
            if let Some(text) = find_string_by_keys(child, keys) {
                return Some(text);
            }
        }
    }

    None
}

fn first_number_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    first_value(value, keys).and_then(value_to_i64)
}

fn first_string_value(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn first_number_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    first_value(value, keys).and_then(value_to_f64)
}

fn first_timestamp(value: &Value, keys: &[&str]) -> Option<i64> {
    first_value(value, keys).and_then(value_to_timestamp)
}

fn first_local_time(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys).and_then(value_to_local_time_text)
}

fn first_value<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|item| item as i64))
        .or_else(|| value.as_str().and_then(|item| item.parse::<i64>().ok()))
}

fn value_to_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|item| item as f64))
        .or_else(|| value.as_str().and_then(|item| item.parse::<f64>().ok()))
}

fn value_to_timestamp(value: &Value) -> Option<i64> {
    let timestamp = value_to_i64(value)?;
    // 接口若返回毫秒时间戳，统一转成秒，避免前端显示到遥远未来。
    if timestamp > 10_000_000_000 {
        Some(timestamp / 1000)
    } else {
        Some(timestamp)
    }
}

fn value_to_local_time_text(value: &Value) -> Option<String> {
    let time = if let Some(timestamp) = value_to_timestamp(value) {
        Utc.timestamp_opt(timestamp, 0).single()?
    } else {
        let text = value.as_str()?;
        DateTime::parse_from_rfc3339(text).ok()?.with_timezone(&Utc)
    };

    Some(
        time.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    )
}

fn normalize_percent(value: f64) -> f64 {
    let percent = if (0.0..=1.0).contains(&value) {
        value * 100.0
    } else {
        value
    };
    percent.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_codex_home_first() {
        let path = resolve_auth_path_from(
            Some(PathBuf::from("X:/CodexHome")),
            Some(PathBuf::from("X:/Users/me")),
        );

        assert_eq!(path, PathBuf::from("X:/CodexHome").join("auth.json"));
    }

    #[test]
    fn resolves_default_codex_auth_path() {
        let path = resolve_auth_path_from(None, Some(PathBuf::from("X:/Users/me")));

        assert_eq!(
            path,
            PathBuf::from("X:/Users/me")
                .join(".codex")
                .join("auth.json")
        );
    }

    #[test]
    fn parses_auth_json_without_exposing_refresh_token() {
        let auth = parse_auth_json(
            r#"{
                "tokens": {
                    "access_token": "access-token-value",
                    "refresh_token": "refresh-token-value"
                },
                "account_id": "acc_123",
                "planType": "Pro"
            }"#,
        )
        .expect("auth parsed");

        assert_eq!(auth.access_token, "access-token-value");
        assert_eq!(auth.account_id.as_deref(), Some("acc_123"));
        assert_eq!(auth.plan_type.as_deref(), Some("Pro"));
    }

    #[test]
    fn missing_token_is_not_logged_in() {
        let error = parse_auth_json(r#"{"refresh_token":"secret"}"#).unwrap_err();

        assert_eq!(error, AuthJsonError::NotLoggedIn);
    }

    #[test]
    fn parses_wham_windows_and_credits() {
        let snapshot = parse_wham_usage(&json!({
            "usage": [
                {
                    "limit_window_seconds": 18000,
                    "used_percent": 28,
                    "reset_at": 1900000000
                },
                {
                    "limit_window_seconds": 604800,
                    "remaining_percent": 88,
                    "reset_at": 1900000000
                }
            ],
            "available_count": 2,
            "credits": [
                {
                    "id": "credit_unique_id_not_returned",
                    "status": "available",
                    "title": "Full reset (Weekly + 5 hr)",
                    "granted_at": "2026-07-03T00:00:00Z",
                    "expires_at": 1900000000
                }
            ],
            "planType": "Pro",
            "rateLimitReachedType": null
        }));

        assert_eq!(
            snapshot.primary_window.unwrap().remaining_percent,
            Some(72.0)
        );
        let secondary = snapshot.secondary_window.as_ref().unwrap();
        assert_eq!(secondary.remaining_percent, Some(88.0));
        assert_eq!(secondary.used_percent, Some(12.0));
        let credits = snapshot.credits.unwrap();
        assert_eq!(credits.available_count, Some(2));
        assert_eq!(credits.items.len(), 1);
        assert_eq!(credits.items[0].status.as_deref(), Some("available"));
        assert_eq!(
            credits.items[0].title.as_deref(),
            Some("Full reset (Weekly + 5 hr)")
        );
        assert!(credits.items[0].granted_at.is_some());
        assert!(credits.items[0].expires_at.is_some());
    }

    #[test]
    fn redacted_errors_do_not_include_sensitive_names() {
        for error in [
            AuthJsonError::NotLoggedIn,
            AuthJsonError::InvalidAuth,
            AuthJsonError::RequestFailed,
        ] {
            let text = error.to_string();
            assert!(!text.contains("access_token"));
            assert!(!text.contains("refresh_token"));
            assert!(!text.contains("Authorization"));
        }
    }
}
