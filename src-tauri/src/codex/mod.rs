mod app_server;
mod auth_json;
mod parser;
mod protocol;
mod reset;
mod session_usage;
mod snapshot;

pub use reset::ResetStats;
pub use snapshot::{
    CodexUsageSnapshot, ResetCreditItem, SnapshotSource, SnapshotStatus, TokenUsage, UsageCredits,
    UsageWindow,
};

use chrono::Local;
use serde_json::Value;

use crate::storage::{current_week_start, AppConfig, StateDocument};

use app_server::CodexAppServer;
use auth_json::{read_auth_json_credits, read_auth_json_snapshot, AuthJsonError};
use parser::{parse_account, parse_rate_limits};
use reset::apply_reset_inference;
use session_usage::read_session_token_usage;

pub fn refresh_codex_snapshot(config: &AppConfig, state: &mut StateDocument) -> CodexUsageSnapshot {
    reset_period_counters(state);

    if config.codex.preferred_provider == "app-server" {
        if let Some(snapshot) = read_app_server_snapshot(config).filter(has_complete_usage) {
            return finalize_snapshot(snapshot, state);
        }
        return read_auth_or_fallback(state);
    }

    match read_auth_json_snapshot() {
        Ok(snapshot) if has_complete_usage(&snapshot) => finalize_snapshot(snapshot, state),
        Err(auth_error) => {
            if let Some(snapshot) = read_app_server_snapshot(config).filter(has_complete_usage) {
                return finalize_snapshot(snapshot, state);
            }
            finalize_snapshot(fallback_snapshot(auth_error), state)
        }
        Ok(snapshot) => {
            if let Some(app_snapshot) = read_app_server_snapshot(config).filter(has_complete_usage)
            {
                return finalize_snapshot(app_snapshot, state);
            }
            finalize_snapshot(snapshot, state)
        }
    }
}

fn read_auth_or_fallback(state: &mut StateDocument) -> CodexUsageSnapshot {
    match read_auth_json_snapshot() {
        Ok(snapshot) => finalize_snapshot(snapshot, state),
        Err(auth_error) => finalize_snapshot(fallback_snapshot(auth_error), state),
    }
}

fn read_app_server_snapshot(config: &AppConfig) -> Option<CodexUsageSnapshot> {
    let mut server = CodexAppServer::start(&config.codex.command).ok()?;
    server.initialize().ok()?;

    let account = server.request("account/read").ok();
    let rate_limits = server.request("account/rateLimits/read").ok()?;
    Some(build_app_server_snapshot(account, rate_limits))
}

fn build_app_server_snapshot(account: Option<Value>, rate_limits: Value) -> CodexUsageSnapshot {
    let mut snapshot = snapshot::empty_snapshot(SnapshotSource::AppServer, SnapshotStatus::Ok);
    let parsed_account = account.as_ref().map(parse_account);

    if let Some(parsed_account) = parsed_account.as_ref() {
        snapshot.plan_type = parsed_account.plan_type.clone();
    }

    let parsed = parse_rate_limits(&rate_limits);
    snapshot.primary_window = parsed.five_hour;
    snapshot.secondary_window = parsed.weekly;
    snapshot.credits = read_auth_json_credits().ok().flatten();
    snapshot.plan_type = parsed.plan_type.or(snapshot.plan_type);
    snapshot.rate_limit_reached_type = parsed.rate_limit_reached_type;
    if snapshot.primary_window.is_none() && snapshot.secondary_window.is_none() {
        snapshot.status = SnapshotStatus::RequestFailed;
    }

    snapshot
}

fn finalize_snapshot(
    mut snapshot: CodexUsageSnapshot,
    state: &mut StateDocument,
) -> CodexUsageSnapshot {
    snapshot.updated_at = Local::now().timestamp();
    apply_reset_inference(state, &snapshot);
    state.last_snapshot = Some(snapshot.clone());
    if state.stats_start_at.is_none() {
        state.stats_start_at = Some(snapshot.updated_at);
    }
    snapshot
}

fn status_for_auth_error(error: AuthJsonError) -> SnapshotStatus {
    error.status()
}

fn fallback_snapshot(auth_error: AuthJsonError) -> CodexUsageSnapshot {
    let source = if read_session_token_usage().is_some() {
        SnapshotSource::SessionLog
    } else {
        SnapshotSource::AuthJson
    };
    snapshot::empty_snapshot(source, status_for_auth_error(auth_error))
}

fn has_complete_usage(snapshot: &CodexUsageSnapshot) -> bool {
    snapshot.status == SnapshotStatus::Ok
        && (snapshot.primary_window.is_some() || snapshot.secondary_window.is_some())
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
