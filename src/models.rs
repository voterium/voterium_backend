use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct Vote {
    pub choice: String,
}

#[derive(Serialize, FromRow)]
pub struct VoteCount {
    pub choice: String,
    pub count: i64,
}
