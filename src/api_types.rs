use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct APIResponse {
    pub date: DateResponse,
    pub games: Vec<GameResponse>,
    pub errors: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateResponse {
    pub raw: String,
    pub pretty: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameResponse {
    pub status: StatusResponse,
    pub start_time: String,
    pub goals: Option<Vec<GoalResponse>>,
    pub scores: HashMap<String, serde_json::Value>,
    pub teams: TeamsResponse,
    pub pre_game_stats: PreGameStatsResponse,
    pub current_stats: CurrentStatsResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub state: String,
    pub progress: Option<ProgressResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressResponse {
    pub current_period: u64,
    pub current_period_ordinal: String,
    pub current_period_time_remaining: CurrentPeriodTimeRemaining,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentPeriodTimeRemaining {
    pub pretty: String,
    pub min: u64,
    pub sec: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scorer {
    pub player: String,
    pub season_total: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assist {
    pub player: String,
    pub season_total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalResponse {
    pub period: String,
    pub scorer: Scorer,
    pub team: String,
    pub assists: Option<Vec<Assist>>,
    pub empty_net: Option<bool>,
    pub min: Option<u64>,
    pub sec: Option<u64>,
    pub strength: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreGameStatsResponse {
    pub records: HashMap<String, serde_json::Value>,
    pub playoff_series: Option<HashMap<String, serde_json::Value>>,
    pub standings: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentStatsResponse {
    pub records: HashMap<String, serde_json::Value>,
    pub streaks: Option<HashMap<String, serde_json::Value>>,
    pub standings: HashMap<String, serde_json::Value>,
    pub playoff_series: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamsResponse {
    pub away: TeamResponse,
    pub home: TeamResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamResponse {
    pub abbreviation: String,
    pub id: u64,
    pub location_name: String,
    pub short_name: String,
    pub team_name: String,
}
