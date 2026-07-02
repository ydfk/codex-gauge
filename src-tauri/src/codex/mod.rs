mod app_server;
mod parser;
mod protocol;
mod reset;
mod session_usage;
mod snapshot;

pub use reset::ResetStats;
pub use snapshot::{CodexGaugeSnapshot, TokenUsage, UsageWindow};

use chrono::Local;
use serde_json::Value;

use crate::storage::{current_week_start, AppConfig, StateDocument};

use app_server::CodexAppServer;
use parser::{parse_account, parse_rate_limits, parse_usage};
use reset::apply_reset_inference;
use session_usage::read_session_token_usage;
use snapshot::{empty_snapshot, SnapshotStatus};

pub fn refresh_codex_snapshot(config: &AppConfig, state: &mut StateDocument) -> CodexGaugeSnapshot {
    reset_period_counters(state);

    let mut snapshot = empty_snapshot();
    let session_token_usage = read_session_token_usage();
    let Ok(mut server) = CodexAppServer::start(&config.codex.command) else {
        if let Some(token_usage) = session_token_usage {
            snapshot.token_usage = token_usage;
            snapshot.status = SnapshotStatus::Partial;
            snapshot.source = "codex-session-log".to_string();
        } else {
            snapshot.status = SnapshotStatus::CodexNotFound;
        }
        snapshot.last_updated_at = Local::now().timestamp();
        state.last_snapshot = Some(snapshot.clone());
        return snapshot;
    };

    if server.initialize().is_err() {
        if let Some(token_usage) = session_token_usage {
            snapshot.token_usage = token_usage;
            snapshot.status = SnapshotStatus::Partial;
            snapshot.source = "codex-session-log".to_string();
        } else {
            snapshot.status = SnapshotStatus::AppServerError;
        }
        snapshot.last_updated_at = Local::now().timestamp();
        state.last_snapshot = Some(snapshot.clone());
        return snapshot;
    }

    let account = server.request("account/read");
    let rate_limits = server.request("account/rateLimits/read");
    let usage = if config.codex.enable_usage_read {
        server.request("account/usage/read").ok()
    } else {
        None
    };

    snapshot = build_snapshot(
        account.ok(),
        rate_limits.ok(),
        usage,
        session_token_usage,
        state,
    );
    state.last_snapshot = Some(snapshot.clone());
    if state.stats_start_at.is_none() {
        state.stats_start_at = Some(snapshot.last_updated_at);
    }

    snapshot
}

fn build_snapshot(
    account: Option<Value>,
    rate_limits: Option<Value>,
    usage: Option<Value>,
    fallback_usage: Option<TokenUsage>,
    state: &mut StateDocument,
) -> CodexGaugeSnapshot {
    let mut snapshot = empty_snapshot();
    let parsed_account = account.as_ref().map(parse_account);

    if let Some(parsed_account) = parsed_account.as_ref() {
        snapshot.account_email = parsed_account.account_email.clone();
        snapshot.plan_type = parsed_account.plan_type.clone();
    }

    if let Some(rate_limits) = rate_limits.as_ref() {
        let parsed = parse_rate_limits(rate_limits);
        snapshot.five_hour = parsed.five_hour;
        snapshot.weekly = parsed.weekly;
        snapshot.other_windows = parsed.other_windows;
        snapshot.reset.available_reset_credits = parsed.available_reset_credits;
        snapshot.plan_type = parsed.plan_type.or(snapshot.plan_type);
        snapshot.credits = parsed.credits;
        snapshot.rate_limit_reached_type = parsed.rate_limit_reached_type;
        snapshot.status = if snapshot.five_hour.is_some() || snapshot.weekly.is_some() {
            SnapshotStatus::Ok
        } else {
            SnapshotStatus::Partial
        };
    } else if parsed_account.is_none() {
        snapshot.status = SnapshotStatus::NotLoggedIn;
    } else {
        snapshot.status = SnapshotStatus::Partial;
    }

    if let Some(usage) = usage.as_ref() {
        snapshot.token_usage = parse_usage(usage);
    }
    if snapshot.token_usage == TokenUsage::default() {
        if let Some(fallback_usage) = fallback_usage {
            snapshot.token_usage = fallback_usage;
            if snapshot.status != SnapshotStatus::Ok {
                snapshot.status = SnapshotStatus::Partial;
            }
        }
    }

    apply_reset_inference(state, &mut snapshot);
    snapshot.last_updated_at = Local::now().timestamp();
    snapshot
}

fn reset_period_counters(state: &mut StateDocument) {
    let today = Local::now().date_naive();
    let today_text = today.format("%Y-%m-%d").to_string();
    let week_start_text = current_week_start().format("%Y-%m-%d").to_string();

    if state.last_stats_date.as_deref() != Some(today_text.as_str()) {
        state.reset_stats.today_auto_reset_5h = 0;
        state.reset_stats.today_auto_reset_weekly = 0;
        state.reset_stats.today_manual_reset_consumed = 0;
        state.last_stats_date = Some(today_text);
    }

    if state.last_stats_week_start.as_deref() != Some(week_start_text.as_str()) {
        state.reset_stats.week_auto_reset_5h = 0;
        state.reset_stats.week_auto_reset_weekly = 0;
        state.reset_stats.week_manual_reset_consumed = 0;
        state.last_stats_week_start = Some(week_start_text);
    }
}
