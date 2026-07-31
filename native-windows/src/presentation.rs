use chrono::{Local, TimeZone};
use slint::{ModelRc, VecModel};

use crate::{
    model::{CodexUsageSnapshot, ResetCreditItem, SnapshotSource, SnapshotStatus, UsageWindow},
    CreditDisplay, GaugeData,
};

pub fn gauge_data(snapshot: &CodexUsageSnapshot) -> GaugeData {
    let five = snapshot.primary_window.as_ref();
    let seven = snapshot.secondary_window.as_ref();
    let five_unlimited = snapshot.primary_window_unlimited;
    let credit_items: Vec<CreditDisplay> = snapshot
        .credits
        .as_ref()
        .map(|credits| credits.items.iter().take(2).map(credit_display).collect())
        .unwrap_or_default();
    let credit_total = snapshot
        .credits
        .as_ref()
        .map(|credits| credits.items.len())
        .unwrap_or(0);

    GaugeData {
        status_text: status_text(&snapshot.status).into(),
        login_needed: matches!(
            snapshot.status,
            SnapshotStatus::NotLoggedIn | SnapshotStatus::InvalidAuth
        ),
        plan_text: snapshot.plan_type.as_deref().unwrap_or("未知").into(),
        source_text: source_text(&snapshot.source).into(),
        five_used_text: if five_unlimited {
            "当前套餐不受限制".into()
        } else {
            used_text(five).into()
        },
        five_remaining_text: if five_unlimited {
            "无限".into()
        } else {
            percent(five.and_then(|value| value.remaining_percent)).into()
        },
        five_reset_text: if five_unlimited {
            "无需重置".into()
        } else {
            reset_time(five.and_then(|value| value.reset_at)).into()
        },
        five_remaining: five
            .and_then(|value| value.remaining_percent)
            .unwrap_or(if five_unlimited { 100.0 } else { 0.0 }) as f32,
        five_visible: !five_unlimited,
        five_unlimited,
        seven_used_text: used_text(seven).into(),
        seven_remaining_text: percent(seven.and_then(|value| value.remaining_percent)).into(),
        seven_reset_text: reset_time(seven.and_then(|value| value.reset_at)).into(),
        seven_remaining: seven
            .and_then(|value| value.remaining_percent)
            .unwrap_or(0.0) as f32,
        reset_count_text: snapshot
            .credits
            .as_ref()
            .and_then(|credits| credits.available_count)
            .map(|count| count.to_string())
            .unwrap_or_else(|| "未知".to_string())
            .into(),
        credit_total_text: format!("共 {credit_total} 张").into(),
        credit_items: ModelRc::new(VecModel::from(credit_items)),
        updated_text: timestamp(snapshot.updated_at).into(),
    }
}

fn credit_display(item: &ResetCreditItem) -> CreditDisplay {
    let (status, available) = credit_status(item.status.as_deref());
    CreditDisplay {
        title: credit_title(item.title.as_deref()).into(),
        status: status.into(),
        expiry: item
            .expires_at
            .as_deref()
            .map(|value| format!("到期 {value}"))
            .unwrap_or_else(|| "未提供到期时间".to_string())
            .into(),
        available,
    }
}

fn credit_title(title: Option<&str>) -> String {
    match title {
        Some(value) if value.to_ascii_lowercase().contains("full reset") => {
            "完整重置（5h + 7d）".to_string()
        }
        Some(value) if !value.trim().is_empty() => value.to_string(),
        _ => "重置券".to_string(),
    }
}

fn credit_status(status: Option<&str>) -> (String, bool) {
    match status.map(str::to_ascii_lowercase).as_deref() {
        Some("available" | "active") => ("可用".to_string(), true),
        Some("used" | "consumed") => ("已使用".to_string(), false),
        Some("expired") => ("已过期".to_string(), false),
        Some(value) => (value.to_string(), false),
        None => ("未知".to_string(), false),
    }
}

pub fn tray_tooltip(snapshot: &CodexUsageSnapshot) -> String {
    let data = gauge_data(snapshot);
    let five = if data.five_unlimited {
        "5h 无限".to_string()
    } else {
        format!(
            "5h 剩{} · {} · 重置{}",
            data.five_remaining_text, data.five_used_text, data.five_reset_text
        )
    };
    format!(
        "Codex Gauge\n{five}\n7d 剩{} · {} · 重置{}\n可用重置 {}",
        data.seven_remaining_text,
        data.seven_used_text,
        data.seven_reset_text,
        data.reset_count_text
    )
}

fn used_text(window: Option<&UsageWindow>) -> String {
    match window.and_then(|value| value.used_percent) {
        Some(value) => format!("已用 {:.0}%", value.clamp(0.0, 100.0)),
        None => "已用 未知".to_string(),
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
        .map(|time| time.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "未知".to_string())
}

fn status_text(status: &SnapshotStatus) -> &'static str {
    match status {
        SnapshotStatus::Ok => "用量状态正常",
        SnapshotStatus::NotLoggedIn => "未检测到 Codex 登录状态",
        SnapshotStatus::InvalidAuth => "Codex 凭据无效，请重新登录",
        SnapshotStatus::RequestFailed => "Codex 用量查询失败",
    }
}

fn source_text(source: &SnapshotSource) -> &'static str {
    match source {
        SnapshotSource::AppServer => "App Server",
        SnapshotSource::AuthJson => "AuthJson API",
    }
}

#[cfg(test)]
mod tests {
    use slint::Model;

    use super::*;
    use crate::model::UsageCredits;

    fn snapshot() -> CodexUsageSnapshot {
        CodexUsageSnapshot {
            source: SnapshotSource::AppServer,
            status: SnapshotStatus::Ok,
            plan_type: Some("plus".to_string()),
            primary_window: None,
            primary_window_unlimited: true,
            secondary_window: Some(UsageWindow {
                name: "weekly".to_string(),
                used_percent: Some(79.0),
                remaining_percent: Some(21.0),
                reset_at: None,
                window_duration_seconds: Some(604_800),
            }),
            credits: None,
            rate_limit_reached_type: None,
            updated_at: 0,
        }
    }

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
    fn unlimited_window_has_explicit_copy() {
        let data = gauge_data(&snapshot());
        assert!(data.five_unlimited);
        assert_eq!(data.five_remaining_text, "无限");
        assert_eq!(data.five_reset_text, "无需重置");
    }

    #[test]
    fn reset_count_is_raw_value_and_credit_list_is_limited() {
        let mut value = snapshot();
        value.credits = Some(UsageCredits {
            available_count: Some(3),
            reset_at: None,
            items: (0..3)
                .map(|_| ResetCreditItem {
                    status: Some("available".to_string()),
                    title: Some("Full reset (Weekly + 5 hr)".to_string()),
                    granted_at: None,
                    expires_at: None,
                })
                .collect(),
        });

        let data = gauge_data(&value);

        assert_eq!(data.reset_count_text, "3");
        assert_eq!(data.credit_total_text, "共 3 张");
        assert_eq!(data.credit_items.row_count(), 2);
        assert_eq!(
            data.credit_items.row_data(0).unwrap().title,
            "完整重置（5h + 7d）"
        );
    }
}
