use jsonwebtoken::DecodingKey;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::vote_logger::{CountsCacheMsg, VLCLMessage};

#[derive(Deserialize)]
pub struct Vote {
    pub choice: String,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct VoteCount {
    pub choice: String,
    pub count: u32,
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
    pub logging_channel_sender: Sender<VLCLMessage>,
    pub cache_channel_sender: Sender<CountsCacheMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub salt: String,
    pub exp: usize,
}
