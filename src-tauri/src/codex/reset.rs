use serde::{Deserialize, Serialize};

use crate::storage::StateDocument;

use super::{CodexGaugeSnapshot, UsageWindow};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResetStats {
    pub available_reset_credits: Option<i64>,
    pub today_auto_reset_5h: u32,
    pub week_auto_reset_5h: u32,
    pub total_auto_reset_5h: u32,
    pub today_auto_reset_weekly: u32,
    pub week_auto_reset_weekly: u32,
    pub total_auto_reset_weekly: u32,
    pub today_manual_reset_consumed: u32,
    pub week_manual_reset_consumed: u32,
    pub total_manual_reset_consumed: u32,
    pub last_available_reset_credits: Option<i64>,
}

impl Default for ResetStats {
    fn default() -> Self {
        Self {
            available_reset_credits: None,
            today_auto_reset_5h: 0,
            week_auto_reset_5h: 0,
            total_auto_reset_5h: 0,
            today_auto_reset_weekly: 0,
            week_auto_reset_weekly: 0,
            total_auto_reset_weekly: 0,
            today_manual_reset_consumed: 0,
            week_manual_reset_consumed: 0,
            total_manual_reset_consumed: 0,
            last_available_reset_credits: None,
        }
    }
}

pub fn apply_reset_inference(state: &mut StateDocument, snapshot: &mut CodexGaugeSnapshot) {
    let available_reset_credits = snapshot.reset.available_reset_credits;
    snapshot.reset = state.reset_stats.clone();
    snapshot.reset.available_reset_credits = available_reset_credits;

    let previous = state.last_snapshot.clone();
    if let Some(previous) = previous.as_ref() {
        record_natural_reset(
            &mut snapshot.reset,
            previous.five_hour.as_ref(),
            snapshot.five_hour.as_ref(),
        );
        record_natural_reset(
            &mut snapshot.reset,
            previous.weekly.as_ref(),
            snapshot.weekly.as_ref(),
        );
        if should_record_manual_reset(previous, snapshot) {
            snapshot.reset.today_manual_reset_consumed += 1;
            snapshot.reset.week_manual_reset_consumed += 1;
            snapshot.reset.total_manual_reset_consumed += 1;
        }
    }

    snapshot.reset.last_available_reset_credits = snapshot.reset.available_reset_credits;
    state.reset_stats = snapshot.reset.clone();
}

fn record_natural_reset(
    stats: &mut ResetStats,
    old_window: Option<&UsageWindow>,
    new_window: Option<&UsageWindow>,
) {
    let now = chrono::Local::now().timestamp();
    let Some(old_window) = old_window else { return };
    let Some(new_window) = new_window else { return };
    let Some(old_resets_at) = old_window.resets_at else {
        return;
    };
    let Some(new_resets_at) = new_window.resets_at else {
        return;
    };

    if old_resets_at >= now
        || new_resets_at <= old_resets_at
        || !used_percent_dropped(old_window, new_window, 0.1)
    {
        return;
    }

    match new_window.label.as_str() {
        "5h" => {
            stats.today_auto_reset_5h += 1;
            stats.week_auto_reset_5h += 1;
            stats.total_auto_reset_5h += 1;
        }
        "1w" => {
            stats.today_auto_reset_weekly += 1;
            stats.week_auto_reset_weekly += 1;
            stats.total_auto_reset_weekly += 1;
        }
        _ => {}
    }
}

fn should_record_manual_reset(
    previous: &CodexGaugeSnapshot,
    snapshot: &CodexGaugeSnapshot,
) -> bool {
    let Some(old_count) = previous.reset.available_reset_credits else {
        return false;
    };
    let Some(new_count) = snapshot.reset.available_reset_credits else {
        return false;
    };

    new_count < old_count && any_window_dropped(previous, snapshot)
}

fn any_window_dropped(previous: &CodexGaugeSnapshot, snapshot: &CodexGaugeSnapshot) -> bool {
    option_window_dropped(
        previous.five_hour.as_ref(),
        snapshot.five_hour.as_ref(),
        5.0,
    ) || option_window_dropped(previous.weekly.as_ref(), snapshot.weekly.as_ref(), 5.0)
}

fn used_percent_dropped(
    old_window: &UsageWindow,
    new_window: &UsageWindow,
    min_delta: f64,
) -> bool {
    let Some(old_used) = old_window.used_percent else {
        return false;
    };
    let Some(new_used) = new_window.used_percent else {
        return false;
    };

    old_used - new_used >= min_delta
}

fn option_window_dropped(
    old_window: Option<&UsageWindow>,
    new_window: Option<&UsageWindow>,
    min_delta: f64,
) -> bool {
    let Some(old_window) = old_window else {
        return false;
    };
    let Some(new_window) = new_window else {
        return false;
    };

    used_percent_dropped(old_window, new_window, min_delta)
}
