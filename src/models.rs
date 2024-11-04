use clickhouse::{error::Error as ClickHouseError, Client, Row};
use jsonwebtoken::DecodingKey;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::vote_logger::{ChannelMessage, VLCLMessage};

#[derive(Deserialize)]
pub struct Vote {
    pub choice: String,
}

#[derive(Serialize, Debug)]
pub struct VoteCount {
    pub choice: String,
    pub count: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub key: String,
    pub label: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub choices: Vec<Choice>,
}

#[derive(Clone)]
pub struct AppState {
    pub backend_salt: Vec<u8>,
    pub config: Config,
    pub decoding_key: DecodingKey,
    pub clickhouse_client: Client,
    pub channel_sender: Sender<VLCLMessage>,
    // pub channel_sender_vl: Sender<ChannelMessage>,
    // pub channel_sender_cl: Sender<ChannelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub salt: String,
    pub exp: usize,
}
