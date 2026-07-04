use serde::{Deserialize, Serialize};

use crate::storage::StateDocument;

use super::{CodexUsageSnapshot, UsageWindow};

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

pub fn apply_reset_inference(state: &mut StateDocument, snapshot: &CodexUsageSnapshot) {
    let mut stats = state.reset_stats.clone();
    stats.available_reset_credits = snapshot
        .credits
        .as_ref()
        .and_then(|credits| credits.reset_credits);

    let previous = state.last_snapshot.clone();
    if let Some(previous) = previous.as_ref() {
        record_natural_reset(
            &mut stats,
            previous.primary_window.as_ref(),
            snapshot.primary_window.as_ref(),
        );
        record_natural_reset(
            &mut stats,
            previous.secondary_window.as_ref(),
            snapshot.secondary_window.as_ref(),
        );
        if should_record_manual_reset(previous, snapshot, &stats) {
            stats.today_manual_reset_consumed += 1;
            stats.week_manual_reset_consumed += 1;
            stats.total_manual_reset_consumed += 1;
        }
    }

    stats.last_available_reset_credits = stats.available_reset_credits;
    state.reset_stats = stats;
}

fn record_natural_reset(
    stats: &mut ResetStats,
    old_window: Option<&UsageWindow>,
    new_window: Option<&UsageWindow>,
) {
    let now = chrono::Local::now().timestamp();
    let Some(old_window) = old_window else { return };
    let Some(new_window) = new_window else { return };
    let Some(old_reset_at) = old_window.reset_at else {
        return;
    };
    let Some(new_reset_at) = new_window.reset_at else {
        return;
    };

    if old_reset_at >= now
        || new_reset_at <= old_reset_at
        || !used_percent_dropped(old_window, new_window, 0.1)
    {
        return;
    }

    match new_window.name.as_str() {
        "5h" => {
            stats.today_auto_reset_5h += 1;
            stats.week_auto_reset_5h += 1;
            stats.total_auto_reset_5h += 1;
        }
        "weekly" => {
            stats.today_auto_reset_weekly += 1;
            stats.week_auto_reset_weekly += 1;
            stats.total_auto_reset_weekly += 1;
        }
        _ => {}
    }
}

fn should_record_manual_reset(
    previous: &CodexUsageSnapshot,
    snapshot: &CodexUsageSnapshot,
    stats: &ResetStats,
) -> bool {
    let Some(old_count) = previous
        .credits
        .as_ref()
        .and_then(|credits| credits.reset_credits)
        .or(stats.last_available_reset_credits)
    else {
        return false;
    };
    let Some(new_count) = snapshot
        .credits
        .as_ref()
        .and_then(|credits| credits.reset_credits)
    else {
        return false;
    };

    new_count < old_count && any_window_dropped(previous, snapshot)
}

fn any_window_dropped(previous: &CodexUsageSnapshot, snapshot: &CodexUsageSnapshot) -> bool {
    option_window_dropped(
        previous.primary_window.as_ref(),
        snapshot.primary_window.as_ref(),
        5.0,
    ) || option_window_dropped(
        previous.secondary_window.as_ref(),
        snapshot.secondary_window.as_ref(),
        5.0,
    )
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
