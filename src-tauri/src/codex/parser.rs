use serde_json::Value;

use super::{UsageCredits, UsageWindow};

#[derive(Debug, Clone)]
pub struct ParsedAccount {
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedRateLimits {
    pub five_hour: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub credits: Option<UsageCredits>,
    pub plan_type: Option<String>,
    pub rate_limit_reached_type: Option<String>,
}

pub fn parse_account(value: &Value) -> ParsedAccount {
    let root = value.get("result").unwrap_or(value);
    ParsedAccount {
        plan_type: first_string(root, &["planType", "plan_type", "plan"]),
    }
}

pub fn parse_rate_limits(value: &Value) -> ParsedRateLimits {
    let root = value.get("result").unwrap_or(value);
    let mut windows = collect_rate_limit_windows(root);
    let mut five_hour = None;
    let mut weekly = None;

    for window in windows.drain(..) {
        match window.name.as_str() {
            "5h" if five_hour.is_none() => five_hour = Some(window),
            "weekly" if weekly.is_none() => weekly = Some(window),
            _ => {}
        }
    }

    ParsedRateLimits {
        five_hour,
        weekly,
        credits: root
            .get("rateLimitResetCredits")
            .map(parse_reset_credits)
            .or_else(|| root.get("credits").map(parse_reset_credits)),
        plan_type: first_string(root, &["planType", "plan_type"]),
        rate_limit_reached_type: first_string(root, &["rateLimitReachedType"]),
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
    let window_duration_seconds = window_duration_mins.map(|mins| mins * 60);
    let used_percent = value.get("usedPercent").and_then(value_to_f64);
    let reset_at = value.get("resetsAt").and_then(value_to_i64);
    let remaining_percent = used_percent.map(|used| (100.0 - used).clamp(0.0, 100.0));

    if window_duration_mins.is_none() && used_percent.is_none() && reset_at.is_none() {
        return None;
    }

    Some(UsageWindow {
        name: label_for_duration(window_duration_mins).to_string(),
        used_percent,
        remaining_percent,
        reset_at,
        window_duration_seconds,
    })
}

fn label_for_duration(duration: Option<i64>) -> &'static str {
    match duration {
        Some(240..=360) => "5h",
        Some(9000..=11000) => "weekly",
        _ => "other",
    }
}

fn parse_reset_credits(value: &Value) -> UsageCredits {
    UsageCredits {
        remaining: value.get("remaining").and_then(value_to_i64),
        available_count: value.get("availableCount").and_then(value_to_i64),
        reset_credits: value
            .get("availableCount")
            .or_else(|| value.get("resetCredits"))
            .and_then(value_to_i64),
        reset_at: value
            .get("resetAt")
            .or_else(|| value.get("resetsAt"))
            .and_then(value_to_i64),
        items: Vec::new(),
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
        assert_eq!(parsed.credits.unwrap().reset_credits, Some(2));
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
}
