use chrono::{DateTime, Utc};
use jsonwebtoken::DecodingKey;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::counting::utils::user_id_hash_u128_from_bytes;

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

impl Choice {
    pub fn key_u8(&self) -> u8 {
        self.key.as_bytes()[0]
    }
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
    pub count_channel_sender: Sender<CountWorkerMsg>,
    pub ledger_channel_sender: Sender<LedgerWorkerMsg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub salt: String,
    pub exp: usize,
}

pub struct Ballot {
    pub vote_id: String,
    pub user_id_hash: String,
    pub timestamp: i64,
    pub choice: String,
}

impl Ballot {
    pub fn to_cl_line(&self) -> String {
        format!("{},{},{}\n", self.user_id_hash, self.timestamp, self.choice)
    }

    pub fn to_vl_line(&self) -> String {
        format!("{},{}\n", self.vote_id, self.choice)
    }

    pub fn choice_key_u8(&self) -> u8 {
        self.choice.as_bytes()[0]
    }

    pub fn user_id_hash_u128(&self) -> u128 {
        user_id_hash_u128_from_bytes(self.user_id_hash.as_bytes())
    }
}

pub struct CountWorkerBallot {
    pub choice_key: u8,
    pub user_id_hash: u128,
}

impl From<&Ballot> for CountWorkerBallot {
    fn from(ballot: &Ballot) -> Self {
        Self {
            choice_key: ballot.choice_key_u8(),
            user_id_hash: ballot.user_id_hash_u128(),
        }
    }
}

pub enum CountWorkerMsg {
    Vote {
        ballot: CountWorkerBallot,
    },
    GetCounts {
        resp: tokio::sync::oneshot::Sender<Vec<VoteCount>>,
    },
}

pub struct LedgerWorkerMsg {
    pub vl_line: Vec<u8>,
    pub cl_line: Vec<u8>,
    pub resp: Option<tokio::sync::oneshot::Sender<bool>>,
}

impl From<&Ballot> for LedgerWorkerMsg {
    fn from(ballot: &Ballot) -> Self {
        Self {
            vl_line: ballot.to_vl_line().into_bytes(),
            cl_line: ballot.to_cl_line().into_bytes(),
            resp: None,
        }
    }
}
