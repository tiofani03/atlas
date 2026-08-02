use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level Connector Health State Taxonomy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorHealthState {
    /// Fully operational (Health Score 90-100)
    Healthy,
    /// Rate limited or experiencing elevated latency/partial failures (Health Score 50-89)
    Degraded,
    /// Auth failure, circuit open, or API endpoint unreachable (Health Score 1-49)
    Unavailable,
    /// Unconfigured or missing credentials (Health Score 0)
    Unconfigured,
}

impl std::fmt::Display for ConnectorHealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectorHealthState::Healthy => write!(f, "HEALTHY"),
            ConnectorHealthState::Degraded => write!(f, "DEGRADED"),
            ConnectorHealthState::Unavailable => write!(f, "UNAVAILABLE"),
            ConnectorHealthState::Unconfigured => write!(f, "UNCONFIGURED"),
        }
    }
}

/// Detailed Health Diagnostics Snapshot for a Connector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub connector_id: String,
    pub provider: String,
    pub state: ConnectorHealthState,
    pub score: u32,
    pub auth_valid: bool,
    pub api_reachable: bool,
    pub p95_latency_ms: u64,
    pub success_rate: f64,
    pub last_checked_at: DateTime<Utc>,
    pub details: String,
}

impl HealthReport {
    pub fn new(
        connector_id: impl Into<String>,
        provider: impl Into<String>,
        auth_valid: bool,
        api_reachable: bool,
        p95_latency_ms: u64,
        success_rate: f64,
        details: impl Into<String>,
    ) -> Self {
        let score = HealthScoreCalculator::calculate(auth_valid, api_reachable, success_rate, p95_latency_ms);
        let state = match score {
            90..=100 => ConnectorHealthState::Healthy,
            50..=89 => ConnectorHealthState::Degraded,
            1..=49 => ConnectorHealthState::Unavailable,
            _ => ConnectorHealthState::Unconfigured,
        };

        Self {
            connector_id: connector_id.into(),
            provider: provider.into(),
            state,
            score,
            auth_valid,
            api_reachable,
            p95_latency_ms,
            success_rate,
            last_checked_at: Utc::now(),
            details: details.into(),
        }
    }
}

/// Composite Health Score Calculator (0 - 100)
pub struct HealthScoreCalculator;

impl HealthScoreCalculator {
    /// Composite formula:
    /// Score = (W_auth * S_auth) + (W_avail * S_avail) + (W_succ * S_succ) + (W_lat * S_lat)
    /// Weights: Auth=0.35, Availability=0.25, SuccessRate=0.25, Latency=0.15
    pub fn calculate(
        auth_valid: bool,
        api_reachable: bool,
        success_rate: f64,
        p95_latency_ms: u64,
    ) -> u32 {
        let auth_score = if auth_valid { 100.0 } else { 0.0 };
        let avail_score = if api_reachable { 100.0 } else { 0.0 };
        let succ_score = success_rate.clamp(0.0, 100.0);
        let latency_score = (100.0 - (p95_latency_ms as f64 / 20.0)).clamp(0.0, 100.0);

        let composite = (0.35 * auth_score) + (0.25 * avail_score) + (0.25 * succ_score) + (0.15 * latency_score);
        composite.round() as u32
    }
}
