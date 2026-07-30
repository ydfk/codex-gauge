use serde_json::Value;

use crate::model::UsageWindow;

#[derive(Debug, Clone)]
pub struct ParsedRateLimits {
    pub five_hour: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub plan_type: Option<String>,
    pub rate_limit_reached_type: Option<String>,
}

pub fn parse_account_plan(value: &Value) -> Option<String> {
    let root = value.get("result").unwrap_or(value);
    find_string(root, &["planType", "plan_type", "plan"])
}

pub fn parse_rate_limits(value: &Value) -> ParsedRateLimits {
    let root = value.get("result").unwrap_or(value);
    let mut windows = collect_windows(root);
    let five_hour = take_named(&mut windows, "5h");
    let weekly = take_named(&mut windows, "weekly");

    ParsedRateLimits {
        five_hour,
        weekly,
        plan_type: find_string(root, &["planType", "plan_type"]),
        rate_limit_reached_type: find_string(root, &["rateLimitReachedType"]),
    }
}

fn collect_windows(root: &Value) -> Vec<UsageWindow> {
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
        if let Some(window) = root.get(key).and_then(parse_usage_window) {
            windows.push(window);
        }
    }
    windows
}

fn collect_windows_from_value(value: &Value, output: &mut Vec<UsageWindow>) {
    if let Some(window) = parse_usage_window(value) {
        output.push(window);
        return;
    }
    if let Some(object) = value.as_object() {
        for child in object.values() {
            collect_windows_from_value(child, output);
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            collect_windows_from_value(child, output);
        }
    }
}

fn parse_usage_window(value: &Value) -> Option<UsageWindow> {
    let duration_mins = value.get("windowDurationMins").and_then(value_to_i64);
    let name = match duration_mins {
        Some(240..=360) => "5h",
        Some(9000..=11000) => "weekly",
        _ => return None,
    };
    let used_percent = value
        .get("usedPercent")
        .and_then(value_to_f64)
        .map(|value| value.clamp(0.0, 100.0));

    Some(UsageWindow {
        name: name.to_string(),
        used_percent,
        remaining_percent: used_percent.map(|used| 100.0 - used),
        reset_at: value.get("resetsAt").and_then(value_to_i64),
        window_duration_seconds: duration_mins.map(|minutes| minutes * 60),
    })
}

fn take_named(windows: &mut Vec<UsageWindow>, name: &str) -> Option<UsageWindow> {
    let index = windows.iter().position(|window| window.name == name)?;
    Some(windows.remove(index))
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

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|value| value as i64))
}

fn value_to_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn prefers_rate_limits_by_id() {
        let parsed = parse_rate_limits(&json!({
            "rateLimitsByLimitId": {
                "short": {"usedPercent": 42, "windowDurationMins": 300},
                "week": {"usedPercent": 17, "windowDurationMins": 10080}
            },
            "rateLimits": [{"usedPercent": 90, "windowDurationMins": 300}]
        }));
        assert_eq!(parsed.five_hour.unwrap().used_percent, Some(42.0));
        assert_eq!(parsed.weekly.unwrap().used_percent, Some(17.0));
    }

    #[test]
    fn falls_back_to_rate_limits() {
        let parsed = parse_rate_limits(&json!({
            "rateLimits": [{"usedPercent": 10, "windowDurationMins": 300}]
        }));
        assert!(parsed.five_hour.is_some());
    }
}
