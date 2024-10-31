use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Vote {
    pub choice: String,
}

#[derive(Serialize)]
pub struct VoteCount {
    pub choice: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub salt: String,
}

#[derive(Clone)]
pub struct CLVote {
    pub user_id_hash: String,
    pub timestamp: i64,
    pub choice: String,
}

#[derive(Clone)]
pub struct VLVote {
    pub vote_id: String,
    pub choice: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Choice {
    pub key: String,
    pub label: String,
    pub color: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub choices: Vec<Choice>,
}

#[derive(Clone)]
pub struct AppState {
    pub backend_salt: Vec<u8>,
    pub config: Config,
}