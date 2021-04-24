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
#[allow(non_snake_case)]
pub struct GameResponse {
    pub status: StatusResponse,
    pub startTime: String,
    pub goals: Vec<GoalResponse>,
    pub scores: HashMap<String, serde_json::Value>,
    pub teams: TeamsResponse,
    pub preGameStats: PreGameStatsResponse,
    pub currentStats: CurrentStatsResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub state: String,
    pub progress: Option<ProgressResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ProgressResponse {
    pub currentPeriod: u64,
    pub currentPeriodOrdinal: String,
    pub currentPeriodTimeRemaining: CurrentPeriodTimeRemaining,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentPeriodTimeRemaining {
    pub pretty: String,
    pub min: u64,
    pub sec: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Scorer {
    pub player: String,
    pub seasonTotal: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Assist {
    pub player: String,
    pub seasonTotal: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct GoalResponse {
    pub period: String,
    pub scorer: Scorer,
    pub team: String,
    pub assists: Option<Vec<Assist>>,
    pub emptyNet: Option<bool>,
    pub min: Option<u64>,
    pub sec: Option<u64>,
    pub strength: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PreGameStatsResponse {
    pub records: HashMap<String, serde_json::Value>,
    pub playoffSeries: Option<HashMap<String, serde_json::Value>>,
    pub standings: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CurrentStatsResponse {
    pub records: HashMap<String, serde_json::Value>,
    pub streaks: HashMap<String, serde_json::Value>,
    pub standings: HashMap<String, serde_json::Value>,
    pub playoffSeries: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamsResponse {
    pub away: TeamResponse,
    pub home: TeamResponse,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TeamResponse {
    pub abbreviation: String,
    pub id: u64,
    pub locationName: String,
    pub shortName: String,
    pub teamName: String,
}
