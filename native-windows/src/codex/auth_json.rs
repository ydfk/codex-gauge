use std::{
    fs,
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

use crate::model::{
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

struct AuthInfo {
    access_token: String,
    account_id: Option<String>,
    plan_type: Option<String>,
}

pub fn read_snapshot() -> Result<CodexUsageSnapshot, AuthJsonError> {
    let auth = read_auth_info(&resolve_auth_path())?;
    fetch_snapshot(&auth)
}

pub fn read_credits() -> Result<Option<UsageCredits>, AuthJsonError> {
    let auth = read_auth_info(&resolve_auth_path())?;
    let client = build_client()?;
    let headers = build_headers(&auth)?;
    request_json(&client, CREDITS_URL, &headers).map(|value| parse_reset_credits(&value))
}

fn resolve_auth_path() -> PathBuf {
    resolve_auth_path_from(
        std::env::var_os("CODEX_HOME").map(PathBuf::from),
        dirs::home_dir(),
    )
}

fn resolve_auth_path_from(codex_home: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    codex_home
        .map(|path| path.join("auth.json"))
        .unwrap_or_else(|| {
            home.unwrap_or_else(|| PathBuf::from("."))
                .join(".codex/auth.json")
        })
}

fn read_auth_info(path: &Path) -> Result<AuthInfo, AuthJsonError> {
    let content = fs::read_to_string(path).map_err(|_| AuthJsonError::NotLoggedIn)?;
    parse_auth_json(&content)
}

fn parse_auth_json(content: &str) -> Result<AuthInfo, AuthJsonError> {
    let value: Value = serde_json::from_str(content).map_err(|_| AuthJsonError::NotLoggedIn)?;
    let access_token = find_string(&value, &["access_token", "accessToken"])
        .filter(|token| !token.trim().is_empty())
        .ok_or(AuthJsonError::NotLoggedIn)?;

    Ok(AuthInfo {
        access_token,
        account_id: find_string(
            &value,
            &[
                "account_id",
                "accountId",
                "chatgpt_account_id",
                "chatgptAccountId",
            ],
        ),
        plan_type: find_string(&value, &["plan_type", "planType", "plan"]),
    })
}

fn fetch_snapshot(auth: &AuthInfo) -> Result<CodexUsageSnapshot, AuthJsonError> {
    let client = build_client()?;
    let headers = build_headers(auth)?;
    let usage = request_json(&client, USAGE_URL, &headers);
    let credits = request_json(&client, CREDITS_URL, &headers);

    if matches!(usage, Err(AuthJsonError::InvalidAuth))
        || matches!(credits, Err(AuthJsonError::InvalidAuth))
    {
        return Err(AuthJsonError::InvalidAuth);
    }

    let parsed_credits = credits.ok().as_ref().and_then(parse_reset_credits);
    let Ok(usage) = usage else {
        let mut snapshot =
            CodexUsageSnapshot::empty(SnapshotSource::AuthJson, SnapshotStatus::RequestFailed);
        snapshot.credits = parsed_credits;
        snapshot.plan_type = auth.plan_type.clone();
        return Ok(snapshot);
    };

    let mut snapshot = parse_wham_usage(&usage);
    snapshot.credits = parsed_credits.or(snapshot.credits);
    snapshot.plan_type = snapshot.plan_type.or_else(|| auth.plan_type.clone());
    Ok(snapshot)
}

fn build_client() -> Result<Client, AuthJsonError> {
    Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| AuthJsonError::RequestFailed)
}

fn build_headers(auth: &AuthInfo) -> Result<HeaderMap, AuthJsonError> {
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_str(&format!("Bearer {}", auth.access_token))
        .map_err(|_| AuthJsonError::InvalidAuth)?;
    headers.insert(AUTHORIZATION, value);
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
    if matches!(
        response.status(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
    ) {
        return Err(AuthJsonError::InvalidAuth);
    }
    if !response.status().is_success() {
        return Err(AuthJsonError::RequestFailed);
    }
    response
        .json::<Value>()
        .map_err(|_| AuthJsonError::RequestFailed)
}

pub fn parse_wham_usage(value: &Value) -> CodexUsageSnapshot {
    let root = value.get("result").unwrap_or(value);
    let mut snapshot = CodexUsageSnapshot::empty(SnapshotSource::AuthJson, SnapshotStatus::Ok);
    let mut windows = Vec::new();
    collect_windows(root, &mut windows);
    for window in windows {
        match window.name.as_str() {
            "5h" if snapshot.primary_window.is_none() => snapshot.primary_window = Some(window),
            "weekly" if snapshot.secondary_window.is_none() => {
                snapshot.secondary_window = Some(window)
            }
            _ => {}
        }
    }
    snapshot.plan_type = find_string(root, &["plan_type", "planType", "plan"]);
    snapshot.rate_limit_reached_type = find_string(root, &["rateLimitReachedType"]);
    snapshot.credits = parse_reset_credits(root);
    snapshot
}

pub fn parse_reset_credits(value: &Value) -> Option<UsageCredits> {
    let root = value.get("result").unwrap_or(value);
    let count_root = root.get("rateLimitResetCredits").unwrap_or(root);
    let mut items = collect_credit_items(root);
    if !std::ptr::eq(root, count_root) {
        items.extend(collect_credit_items(count_root));
    }
    let credits = UsageCredits {
        available_count: first_i64(
            count_root,
            &[
                "available_count",
                "availableCount",
                "available",
                "availableCredits",
            ],
        ),
        reset_at: first_timestamp(count_root, &["resetAt", "reset_at", "resetsAt"]),
        items,
    };
    (credits.available_count.is_some() || credits.reset_at.is_some() || !credits.items.is_empty())
        .then_some(credits)
}

fn collect_credit_items(value: &Value) -> Vec<ResetCreditItem> {
    find_credit_array(value)
        .map(|items| items.iter().filter_map(parse_credit_item).collect())
        .unwrap_or_default()
}

fn find_credit_array(value: &Value) -> Option<&Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }
    let object = value.as_object()?;
    for key in ["credits", "items", "data", "resetCredits", "reset_credits"] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            return Some(items);
        }
    }
    None
}

fn parse_credit_item(value: &Value) -> Option<ResetCreditItem> {
    let item = ResetCreditItem {
        status: first_string(value, &["status", "state"]),
        title: first_string(value, &["title", "displayTitle", "display_title", "name"]),
        granted_at: first_local_time(value, &["granted_at", "grantedAt", "created_at"]),
        expires_at: first_local_time(value, &["expires_at", "expiresAt", "expiration_at"]),
    };
    (item.status.is_some()
        || item.title.is_some()
        || item.granted_at.is_some()
        || item.expires_at.is_some())
    .then_some(item)
}

fn collect_windows(value: &Value, output: &mut Vec<UsageWindow>) {
    if let Some(window) = parse_wham_window(value) {
        output.push(window);
        return;
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            collect_windows(child, output);
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            collect_windows(child, output);
        }
    }
}

fn parse_wham_window(value: &Value) -> Option<UsageWindow> {
    let duration = first_i64(
        value,
        &[
            "limit_window_seconds",
            "limitWindowSeconds",
            "windowDurationSeconds",
        ],
    );
    let name = match duration {
        Some(18_000) => "5h",
        Some(604_800) => "weekly",
        _ => return None,
    };
    let raw_used =
        first_f64(value, &["usedPercent", "used_percent", "usagePercent"]).map(normalize_percent);
    let raw_remaining =
        first_f64(value, &["remainingPercent", "remaining_percent"]).map(normalize_percent);
    let used = raw_used.or_else(|| raw_remaining.map(|remaining| 100.0 - remaining));
    let remaining = raw_remaining.or_else(|| used.map(|value| 100.0 - value));

    Some(UsageWindow {
        name: name.to_string(),
        used_percent: used,
        remaining_percent: remaining,
        reset_at: first_timestamp(value, &["resetAt", "reset_at", "resetsAt", "expiresAt"]),
        window_duration_seconds: duration,
    })
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(object) = value.as_object() {
        for key in keys {
            if let Some(text) = object.get(*key).and_then(Value::as_str) {
                return Some(text.to_string());
            }
        }
        for child in object.values() {
            if let Some(text) = find_string(child, keys) {
                return Some(text);
            }
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            if let Some(text) = find_string(child, keys) {
                return Some(text);
            }
        }
    }
    None
}

fn first_value<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)?.as_str().map(ToString::to_string)
}

fn first_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    let value = first_value(value, keys)?;
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|number| number as i64))
        .or_else(|| value.as_str()?.parse().ok())
}

fn first_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    let value = first_value(value, keys)?;
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
        .or_else(|| value.as_str()?.parse().ok())
}

fn first_timestamp(value: &Value, keys: &[&str]) -> Option<i64> {
    let timestamp = first_i64(value, keys)?;
    Some(if timestamp > 10_000_000_000 {
        timestamp / 1000
    } else {
        timestamp
    })
}

fn first_local_time(value: &Value, keys: &[&str]) -> Option<String> {
    let value = first_value(value, keys)?;
    let time = if let Some(timestamp) = value.as_i64() {
        Utc.timestamp_opt(timestamp, 0).single()?
    } else {
        DateTime::parse_from_rfc3339(value.as_str()?)
            .ok()?
            .with_timezone(&Utc)
    };
    Some(
        time.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    )
}

fn normalize_percent(value: f64) -> f64 {
    if (0.0..=1.0).contains(&value) {
        (value * 100.0).clamp(0.0, 100.0)
    } else {
        value.clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn codex_home_has_priority() {
        let path = resolve_auth_path_from(
            Some(PathBuf::from("X:/CodexHome")),
            Some(PathBuf::from("X:/Users/me")),
        );
        assert_eq!(path, PathBuf::from("X:/CodexHome/auth.json"));
    }

    #[test]
    fn parses_windows_and_reset_credits() {
        let value = json!({
            "rate_limits": {
                "primary": {"limit_window_seconds": 18000, "used_percent": 25},
                "secondary": {"limit_window_seconds": 604800, "used_percent": 70}
            },
            "available_count": 2
        });
        let snapshot = parse_wham_usage(&value);
        assert_eq!(
            snapshot.primary_window.unwrap().remaining_percent,
            Some(75.0)
        );
        assert_eq!(
            snapshot.secondary_window.unwrap().remaining_percent,
            Some(30.0)
        );
        assert_eq!(snapshot.credits.unwrap().available_count, Some(2));
    }

    #[test]
    fn errors_never_include_credentials() {
        for error in [
            AuthJsonError::NotLoggedIn,
            AuthJsonError::InvalidAuth,
            AuthJsonError::RequestFailed,
        ] {
            let text = format!("{error:?}");
            assert!(!text.contains("access_token"));
            assert!(!text.contains("Authorization"));
        }
    }
}
