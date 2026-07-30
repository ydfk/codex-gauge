use chrono::{Local, TimeZone};

use crate::{
    model::{CodexUsageSnapshot, SnapshotSource, SnapshotStatus, UsageWindow},
    GaugeData,
};

pub fn gauge_data(snapshot: &CodexUsageSnapshot) -> GaugeData {
    let five = snapshot.primary_window.as_ref();
    let seven = snapshot.secondary_window.as_ref();
    let five_unlimited = snapshot.primary_window_unlimited;
    let credit = snapshot
        .credits
        .as_ref()
        .and_then(|credits| credits.items.first());

    GaugeData {
        status_text: status_text(&snapshot.status).into(),
        plan_text: snapshot.plan_type.as_deref().unwrap_or("未知").into(),
        source_text: source_text(&snapshot.source).into(),
        five_used_text: if five_unlimited {
            "5h 不限量".into()
        } else {
            used_text("5h", five).into()
        },
        five_remaining_text: if five_unlimited {
            "无限".into()
        } else {
            percent(five.and_then(|value| value.remaining_percent)).into()
        },
        five_reset_text: if five_unlimited {
            "不限量".into()
        } else {
            reset_time(five.and_then(|value| value.reset_at)).into()
        },
        five_remaining: five
            .and_then(|value| value.remaining_percent)
            .unwrap_or(if five_unlimited { 100.0 } else { 0.0 }) as f32,
        five_visible: !five_unlimited,
        seven_used_text: used_text("7d", seven).into(),
        seven_remaining_text: percent(seven.and_then(|value| value.remaining_percent)).into(),
        seven_reset_text: reset_time(seven.and_then(|value| value.reset_at)).into(),
        seven_remaining: seven
            .and_then(|value| value.remaining_percent)
            .unwrap_or(0.0) as f32,
        reset_count_text: snapshot
            .credits
            .as_ref()
            .and_then(|credits| credits.available_count)
            .map(|count| format!("重置 {count}"))
            .unwrap_or_else(|| "重置未知".to_string())
            .into(),
        credit_detail_text: credit
            .map(|item| credit_detail(item.title.as_deref(), item.status.as_deref()))
            .unwrap_or_else(|| "未知".to_string())
            .into(),
        credit_expiry_text: credit
            .and_then(|item| item.expires_at.as_deref())
            .unwrap_or("未知")
            .into(),
        updated_text: timestamp(snapshot.updated_at).into(),
    }
}

fn credit_detail(title: Option<&str>, status: Option<&str>) -> String {
    let title = match title {
        Some(value) if value.starts_with("Full reset") => "7d + 5h 完整重置",
        Some(value) => value,
        None => "未知类型",
    };
    let status = match status {
        Some("available") => "可用",
        Some("used") => "已使用",
        Some("expired") => "已过期",
        Some(value) => value,
        None => "未知状态",
    };
    format!("{title} · {status}")
}

pub fn tray_tooltip(snapshot: &CodexUsageSnapshot) -> String {
    let data = gauge_data(snapshot);
    let five = if data.five_visible {
        format!(
            "5h 剩{} · {} · 重置{}",
            data.five_remaining_text, data.five_used_text, data.five_reset_text
        )
    } else {
        "5h 无限".to_string()
    };
    format!(
        "Codex Gauge\n{five}\n7d 剩{} · {} · 重置{}\n{}",
        data.seven_remaining_text,
        data.seven_used_text,
        data.seven_reset_text,
        data.reset_count_text
    )
}

fn used_text(label: &str, window: Option<&UsageWindow>) -> String {
    match window.and_then(|value| value.used_percent) {
        Some(value) => format!("{label}已用 {:.0}%", value.clamp(0.0, 100.0)),
        None => format!("{label}已用 未知"),
    }
}

fn percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.0}%", value.clamp(0.0, 100.0)))
        .unwrap_or_else(|| "未知".to_string())
}

fn reset_time(value: Option<i64>) -> String {
    value
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single())
        .map(|time| time.format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "未知".to_string())
}

fn timestamp(value: i64) -> String {
    Local
        .timestamp_opt(value, 0)
        .single()
        .map(|time| time.format("%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "未知".to_string())
}

fn status_text(status: &SnapshotStatus) -> &'static str {
    match status {
        SnapshotStatus::Ok => "正常",
        SnapshotStatus::NotLoggedIn => "未检测到 Codex 登录状态",
        SnapshotStatus::InvalidAuth => "Codex 凭据无效，请重新登录",
        SnapshotStatus::RequestFailed => "Codex 用量查询失败",
    }
}

fn source_text(source: &SnapshotSource) -> &'static str {
    match source {
        SnapshotSource::AppServer => "app-server",
        SnapshotSource::AuthJson => "API",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_remain_unknown() {
        assert_eq!(percent(None), "未知");
        assert_eq!(reset_time(None), "未知");
    }

    #[test]
    fn percent_is_clamped() {
        assert_eq!(percent(Some(120.0)), "100%");
        assert_eq!(percent(Some(-2.0)), "0%");
    }

    #[test]
    fn localizes_full_reset_credit() {
        assert_eq!(
            credit_detail(Some("Full reset"), Some("available")),
            "7d + 5h 完整重置 · 可用"
        );
    }
}
