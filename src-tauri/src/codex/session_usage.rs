use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, NaiveDate};
use serde_json::Value;

use super::TokenUsage;
use crate::storage::current_week_start;

#[derive(Debug, Clone, Copy, Default)]
struct UsageTotals {
    input_tokens: i64,
    cached_input_tokens: i64,
    cache_creation_input_tokens: i64,
    output_tokens: i64,
    reasoning_output_tokens: i64,
    total_tokens: i64,
}

pub fn read_session_token_usage() -> Option<TokenUsage> {
    let codex_home = codex_home_dir()?;
    let mut files = Vec::new();

    collect_jsonl_files(&codex_home.join("sessions"), &mut files);
    collect_jsonl_files(&codex_home.join("archived_sessions"), &mut files);
    files.sort();

    let mut daily = BTreeMap::<NaiveDate, i64>::new();
    for file in files {
        read_file_usage(&file, &mut daily);
    }

    if daily.is_empty() {
        return None;
    }

    let today = Local::now().date_naive();
    let week_start = current_week_start();
    let today_tokens = daily.get(&today).copied();
    let week_tokens = daily
        .range(week_start..=today)
        .map(|(_, tokens)| *tokens)
        .sum::<i64>();
    let lifetime_tokens = daily.values().sum::<i64>();
    let peak_daily_tokens = daily.values().copied().max();

    Some(TokenUsage {
        today_tokens,
        week_tokens: (week_tokens > 0).then_some(week_tokens),
        lifetime_tokens: Some(lifetime_tokens),
        peak_daily_tokens,
    })
}

fn codex_home_dir() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))
}

fn collect_jsonl_files(dir: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, output);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            output.push(path);
        }
    }
}

fn read_file_usage(path: &Path, daily: &mut BTreeMap<NaiveDate, i64>) {
    let Ok(file) = File::open(path) else {
        return;
    };

    let mut previous_total: Option<UsageTotals> = None;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some((date, delta)) = parse_token_count_delta(&value, previous_total) else {
            continue;
        };

        if let Some(total) = extract_total_usage(&value) {
            previous_total = Some(total);
        }

        if delta.total_tokens > 0 {
            *daily.entry(date).or_default() += delta.total_tokens;
        }
    }
}

fn parse_token_count_delta(
    value: &Value,
    previous_total: Option<UsageTotals>,
) -> Option<(NaiveDate, UsageTotals)> {
    let payload = value.get("payload")?;
    if value.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }

    let info = if payload.get("type").and_then(Value::as_str) == Some("token_count") {
        payload.get("info")
    } else if payload.pointer("/msg/type").and_then(Value::as_str) == Some("token_count") {
        payload.pointer("/msg/info")
    } else {
        None
    }?;

    let date = value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_local_date)?;
    let last = info.get("last_token_usage").map(read_usage_totals);
    let total = info.get("total_token_usage").map(read_usage_totals);

    let delta = match (last, total, previous_total) {
        (_, Some(total), Some(previous)) if total.total_tokens >= previous.total_tokens => {
            total.saturating_sub(previous)
        }
        (Some(last), _, _) => last,
        (_, Some(total), _) => total,
        _ => return None,
    };

    Some((date, normalize_totals(delta)))
}

fn extract_total_usage(value: &Value) -> Option<UsageTotals> {
    let payload = value.get("payload")?;
    let info = if payload.get("type").and_then(Value::as_str) == Some("token_count") {
        payload.get("info")
    } else if payload.pointer("/msg/type").and_then(Value::as_str) == Some("token_count") {
        payload.pointer("/msg/info")
    } else {
        None
    }?;

    info.get("total_token_usage").map(read_usage_totals)
}

fn read_usage_totals(value: &Value) -> UsageTotals {
    UsageTotals {
        input_tokens: number_at(value, "input_tokens"),
        cached_input_tokens: number_at(value, "cached_input_tokens"),
        cache_creation_input_tokens: number_at(value, "cache_creation_input_tokens"),
        output_tokens: number_at(value, "output_tokens"),
        reasoning_output_tokens: number_at(value, "reasoning_output_tokens"),
        total_tokens: number_at(value, "total_tokens"),
    }
}

fn normalize_totals(mut usage: UsageTotals) -> UsageTotals {
    usage.input_tokens = (usage.input_tokens - usage.cached_input_tokens).max(0);
    usage.total_tokens = usage.input_tokens
        + usage.cached_input_tokens
        + usage.cache_creation_input_tokens
        + usage.output_tokens
        + usage.reasoning_output_tokens;
    usage
}

fn number_at(value: &Value, key: &str) -> i64 {
    value
        .get(key)
        .and_then(|item| item.as_i64().or_else(|| item.as_f64().map(|n| n as i64)))
        .unwrap_or(0)
        .max(0)
}

fn parse_local_date(value: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Local).date_naive())
}

impl UsageTotals {
    fn saturating_sub(self, previous: UsageTotals) -> UsageTotals {
        UsageTotals {
            input_tokens: (self.input_tokens - previous.input_tokens).max(0),
            cached_input_tokens: (self.cached_input_tokens - previous.cached_input_tokens).max(0),
            cache_creation_input_tokens: (self.cache_creation_input_tokens
                - previous.cache_creation_input_tokens)
                .max(0),
            output_tokens: (self.output_tokens - previous.output_tokens).max(0),
            reasoning_output_tokens: (self.reasoning_output_tokens
                - previous.reasoning_output_tokens)
                .max(0),
            total_tokens: (self.total_tokens - previous.total_tokens).max(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_nested_token_count_delta() {
        let value = json!({
            "timestamp": "2026-07-02T01:02:03Z",
            "type": "event_msg",
            "payload": {
                "msg": {
                    "type": "token_count",
                    "info": {
                        "last_token_usage": {
                            "input_tokens": 100,
                            "cached_input_tokens": 20,
                            "output_tokens": 30
                        }
                    }
                }
            }
        });

        let (_, usage) = parse_token_count_delta(&value, None).expect("token count");

        assert_eq!(usage.total_tokens, 130);
    }
}
