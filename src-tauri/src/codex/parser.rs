use chrono::{Datelike, Local, NaiveDate};
use serde_json::Value;

use super::{TokenUsage, UsageWindow};

#[derive(Debug, Clone)]
pub struct ParsedAccount {
    pub account_email: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedRateLimits {
    pub five_hour: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub other_windows: Vec<UsageWindow>,
    pub available_reset_credits: Option<i64>,
    pub plan_type: Option<String>,
    pub credits: Option<Value>,
    pub rate_limit_reached_type: Option<String>,
}

pub fn parse_account(value: &Value) -> ParsedAccount {
    let root = value.get("result").unwrap_or(value);
    ParsedAccount {
        account_email: first_string(root, &["email", "accountEmail", "account.email"]),
        plan_type: first_string(root, &["planType", "plan_type", "plan"]),
    }
}

pub fn parse_rate_limits(value: &Value) -> ParsedRateLimits {
    let root = value.get("result").unwrap_or(value);
    let mut windows = collect_rate_limit_windows(root);
    let mut five_hour = None;
    let mut weekly = None;
    let mut other_windows = Vec::new();

    for window in windows.drain(..) {
        match window.label.as_str() {
            "5h" if five_hour.is_none() => five_hour = Some(window),
            "1w" if weekly.is_none() => weekly = Some(window),
            _ => other_windows.push(window),
        }
    }

    ParsedRateLimits {
        five_hour,
        weekly,
        other_windows,
        available_reset_credits: root
            .pointer("/rateLimitResetCredits/availableCount")
            .and_then(value_to_i64),
        plan_type: first_string(root, &["planType", "plan_type"]),
        credits: root.get("credits").cloned(),
        rate_limit_reached_type: first_string(root, &["rateLimitReachedType"]),
    }
}

pub fn parse_usage(value: &Value) -> TokenUsage {
    let root = value.get("result").unwrap_or(value);
    let summary = root.get("summary").unwrap_or(root);
    let today = Local::now().date_naive();
    let week_start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let mut today_tokens = None;
    let mut week_tokens = 0_i64;
    let mut has_week_tokens = false;

    if let Some(buckets) = root.get("dailyUsageBuckets").and_then(Value::as_array) {
        for bucket in buckets {
            let Some(date) = bucket
                .get("startDate")
                .and_then(Value::as_str)
                .and_then(parse_date)
            else {
                continue;
            };
            let Some(tokens) = bucket.get("tokens").and_then(value_to_i64) else {
                continue;
            };

            if date == today {
                today_tokens = Some(tokens);
            }

            if date >= week_start && date <= today {
                week_tokens += tokens;
                has_week_tokens = true;
            }
        }
    }

    TokenUsage {
        today_tokens,
        week_tokens: has_week_tokens.then_some(week_tokens),
        lifetime_tokens: summary.get("lifetimeTokens").and_then(value_to_i64),
        peak_daily_tokens: summary.get("peakDailyTokens").and_then(value_to_i64),
    }
}

fn collect_rate_limit_windows(root: &Value) -> Vec<UsageWindow> {
    let mut windows = Vec::new();

    if let Some(by_id) = root.get("rateLimitsByLimitId") {
        collect_windows_from_value(by_id, &mut windows);
    }

    if windows.is_empty() {
        if let Some(rate_limits) = root.get("rateLimits") {
            collect_windows_from_value(rate_limits, &mut windows);
        }
    }

    for key in ["primary", "secondary"] {
        if let Some(value) = root.get(key) {
            if let Some(window) = parse_usage_window(value) {
                windows.push(window);
            }
        }
    }

    windows
}

fn collect_windows_from_value(value: &Value, output: &mut Vec<UsageWindow>) {
    if let Some(window) = parse_usage_window(value) {
        output.push(window);
        return;
    }

    if let Some(primary) = value.get("primary") {
        if let Some(window) = parse_usage_window(primary) {
            output.push(window);
        }
    }
    if let Some(secondary) = value.get("secondary") {
        if let Some(window) = parse_usage_window(secondary) {
            output.push(window);
        }
    }

    if let Some(rate_limits) = value.get("rateLimits") {
        collect_windows_from_value(rate_limits, output);
        return;
    }

    if let Some(values) = value.as_object() {
        for item in values.values() {
            collect_windows_from_value(item, output);
        }
        return;
    }

    if let Some(values) = value.as_array() {
        for item in values {
            collect_windows_from_value(item, output);
        }
    }
}

fn parse_usage_window(value: &Value) -> Option<UsageWindow> {
    let window_duration_mins = value.get("windowDurationMins").and_then(value_to_i64);
    let used_percent = value.get("usedPercent").and_then(value_to_f64);
    let resets_at = value.get("resetsAt").and_then(value_to_i64);
    let remaining_percent = used_percent.map(|used| (100.0 - used).clamp(0.0, 100.0));

    if window_duration_mins.is_none() && used_percent.is_none() && resets_at.is_none() {
        return None;
    }

    Some(UsageWindow {
        label: label_for_duration(window_duration_mins).to_string(),
        used_percent,
        remaining_percent,
        window_duration_mins,
        resets_at,
        reset_remaining_text: resets_at.map(format_reset_remaining),
    })
}

fn label_for_duration(duration: Option<i64>) -> &'static str {
    match duration {
        Some(240..=360) => "5h",
        Some(9000..=11000) => "1w",
        _ => "other",
    }
}

fn format_reset_remaining(resets_at: i64) -> String {
    let now = Local::now().timestamp();
    let remaining = (resets_at - now).max(0);
    let days = remaining / 86_400;
    let hours = (remaining % 86_400) / 3_600;
    let mins = (remaining % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours:02}h")
    } else {
        format!("{hours:02}:{mins:02}")
    }
}

fn first_string(root: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value_at(root, key).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn value_at<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    if key.contains('.') {
        let pointer = format!("/{}", key.replace('.', "/"));
        root.pointer(&pointer)
    } else {
        root.get(key)
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|item| item as i64))
}

fn value_to_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|item| item as f64))
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&value[..value.len().min(10)], "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_rate_limits_by_limit_id_first() {
        let parsed = parse_rate_limits(&json!({
            "result": {
                "rateLimitsByLimitId": {
                    "short": { "usedPercent": 42, "windowDurationMins": 300, "resetsAt": 1900000000 },
                    "week": { "usedPercent": 17, "windowDurationMins": 10080, "resetsAt": 1900000000 }
                },
                "rateLimits": [
                    { "usedPercent": 90, "windowDurationMins": 300, "resetsAt": 1900000000 }
                ],
                "rateLimitResetCredits": { "availableCount": 2 }
            }
        }));

        assert_eq!(parsed.five_hour.unwrap().used_percent, Some(42.0));
        assert_eq!(parsed.weekly.unwrap().used_percent, Some(17.0));
        assert_eq!(parsed.available_reset_credits, Some(2));
    }

    #[test]
    fn falls_back_to_rate_limits() {
        let parsed = parse_rate_limits(&json!({
            "rateLimits": [
                { "usedPercent": 10, "windowDurationMins": 300, "resetsAt": 1900000000 }
            ]
        }));

        assert!(parsed.five_hour.is_some());
        assert!(parsed.weekly.is_none());
    }

    #[test]
    fn falls_back_when_rate_limits_by_id_has_no_windows() {
        let parsed = parse_rate_limits(&json!({
            "rateLimitsByLimitId": {
                "codex": {}
            },
            "rateLimits": {
                "primary": { "usedPercent": 12, "windowDurationMins": 300, "resetsAt": 1900000000 },
                "secondary": { "usedPercent": 27, "windowDurationMins": 10080, "resetsAt": 1900000000 }
            }
        }));

        assert_eq!(parsed.five_hour.unwrap().used_percent, Some(12.0));
        assert_eq!(parsed.weekly.unwrap().used_percent, Some(27.0));
    }

    #[test]
    fn usage_missing_fields_stays_unknown() {
        let usage = parse_usage(&json!({ "summary": {} }));

        assert_eq!(usage.today_tokens, None);
        assert_eq!(usage.week_tokens, None);
        assert_eq!(usage.lifetime_tokens, None);
    }
}
