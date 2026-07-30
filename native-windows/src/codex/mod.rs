mod app_server;
mod auth_json;
mod parser;
mod protocol;

use serde_json::Value;

use crate::{
    config::{AppConfig, ProviderPreference},
    model::{CodexUsageSnapshot, SnapshotSource, SnapshotStatus},
};

use app_server::CodexAppServer;

pub fn refresh_snapshot(config: &AppConfig) -> CodexUsageSnapshot {
    let mut snapshot = match config.preferred_provider {
        ProviderPreference::AppServer => read_app_server(config)
            .filter(has_usage)
            .unwrap_or_else(read_auth_snapshot),
        ProviderPreference::Api => {
            let api = read_auth_snapshot();
            if has_usage(&api) {
                api
            } else {
                read_app_server(config).filter(has_usage).unwrap_or(api)
            }
        }
    };

    if snapshot.credits.is_none() {
        snapshot.credits = auth_json::read_credits().ok().flatten();
    }
    snapshot.normalize();
    snapshot
}

fn read_app_server(config: &AppConfig) -> Option<CodexUsageSnapshot> {
    let mut server = CodexAppServer::start(&config.codex_command).ok()?;
    server.initialize().ok()?;
    let account = server.request("account/read").ok();
    let rate_limits = server.request("account/rateLimits/read").ok()?;
    Some(build_app_server_snapshot(account, rate_limits))
}

fn build_app_server_snapshot(account: Option<Value>, rate_limits: Value) -> CodexUsageSnapshot {
    let mut snapshot = CodexUsageSnapshot::empty(SnapshotSource::AppServer, SnapshotStatus::Ok);
    let parsed = parser::parse_rate_limits(&rate_limits);
    snapshot.primary_window = parsed.five_hour;
    snapshot.secondary_window = parsed.weekly;
    snapshot.plan_type = parsed
        .plan_type
        .or_else(|| account.as_ref().and_then(parser::parse_account_plan));
    snapshot.rate_limit_reached_type = parsed.rate_limit_reached_type;
    if snapshot.primary_window.is_none() && snapshot.secondary_window.is_none() {
        snapshot.status = SnapshotStatus::RequestFailed;
    }
    snapshot
}

fn read_auth_snapshot() -> CodexUsageSnapshot {
    match auth_json::read_snapshot() {
        Err(error) => CodexUsageSnapshot::empty(SnapshotSource::AuthJson, error.status()),
        Ok(snapshot) => snapshot,
    }
}

fn has_usage(snapshot: &CodexUsageSnapshot) -> bool {
    snapshot.status == SnapshotStatus::Ok
        && (snapshot.primary_window.is_some() || snapshot.secondary_window.is_some())
}

#[cfg(test)]
mod tests {
    use crate::model::UsageWindow;

    use super::*;

    #[test]
    fn successful_week_only_snapshot_marks_five_hour_unlimited() {
        let mut snapshot = CodexUsageSnapshot::empty(SnapshotSource::AppServer, SnapshotStatus::Ok);
        snapshot.secondary_window = Some(UsageWindow {
            name: "weekly".to_string(),
            used_percent: Some(10.0),
            remaining_percent: Some(90.0),
            reset_at: None,
            window_duration_seconds: Some(604_800),
        });
        snapshot.normalize();
        assert!(snapshot.primary_window_unlimited);
    }
}
