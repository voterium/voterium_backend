use crate::auth::validate_jwt;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString, PasswordHasher},
    Argon2
};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Vote {
    pub choice: String,
}

#[derive(Serialize)]
pub struct VoteCount {
    pub choice: String,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String,       // user_id
    pub exp: usize,
    pub salt: String, // user's unique salt
}

#[derive(Serialize, Deserialize, Clone)]
struct CLVote {
    pub user_id_hash: String,
    pub timestamp: i64,
    pub choice: String,
}

#[derive(Serialize, Deserialize)]
struct VLVote {
    pub vote_id: String,
    pub choice: String,
}

#[derive(Clone)]
pub struct AppState {
    pub backend_salt: String,
}

fn hash_user_id(user_id: &str, user_salt: &str, backend_salt: &str) -> Result<String, Error> {
    // let combined_id = format!("{}{}{}", user_id, user_salt, backend_salt);
    // let salt = SaltString::generate(&mut OsRng);
    let algo = Argon2::default();

    let user_salt = SaltString::from_b64(&user_salt).map_err(|err| actix_web::error::ErrorInternalServerError(format!("Salt error: {}", err)))?;
    println!("user_id: {}, user_salt: {}, backend_salt: {}", user_id, user_salt, backend_salt);
    let hash1 = algo.hash_password(user_id.as_bytes(), &user_salt).map_err(|err| actix_web::error::ErrorInternalServerError(format!("Hash error: {}", err)))?;

    let backend_salt = SaltString::from_b64(&backend_salt).map_err(|err| actix_web::error::ErrorInternalServerError(format!("Salt error: {}", err)))?;
    let user_id_hash = algo.hash_password(&hash1.to_string().into_bytes(), &backend_salt).map_err(|err| actix_web::error::ErrorInternalServerError(format!("Hash error: {}", err)))?;
    // let user_id_hash = algo
    //     .hash_password(combined_id.as_bytes(), &salt)
    //     .map_err(|err| actix_web::error::ErrorInternalServerError(format!("Hash error: {}", err)))?;
    Ok(user_id_hash.to_string())
}

pub async fn vote(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    // Validate JWT and extract claims
    let claims = validate_jwt(&req).await?;

    let user_id = claims.sub;
    let user_salt = claims.salt;
    let backend_salt = &app_state.backend_salt;
    let vote_id = Uuid::new_v4().to_string();

    // Compute user_id_hash using Argon2
    let user_id_hash = hash_user_id(&user_id, &user_salt, &backend_salt)?;

    // Get current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    // Append to Public Vote Verification Ledger (VL)
    let vl_vote = VLVote {
        vote_id,
        choice: vote.choice.clone(),
    };
    append_to_file("vl.jsonl", &vl_vote)?;

    // Append to Public Vote Count Ledger (CL)
    let cl_vote = CLVote {
        user_id_hash,
        timestamp,
        choice: vote.choice.clone(),
    };
    append_to_file("cl.jsonl", &cl_vote)?;

    Ok(HttpResponse::Ok().body("Vote recorded"))
}

pub async fn results(_req: HttpRequest) -> Result<HttpResponse, Error> {
    // Read the CL file
    let file = File::open("cl.jsonl").map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("File error: {}", err))
    })?;

    let reader = BufReader::new(file);
    let mut votes: Vec<CLVote> = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("File read error: {}", err))
        })?;

        let vote: CLVote = serde_json::from_str(&line).map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("Deserialization error: {}", err))
        })?;

        votes.push(vote);
    }

    // Build a map of user_id_hash to their latest vote
    let mut latest_votes: HashMap<String, CLVote> = HashMap::new();
    for vote in votes {
        latest_votes
            .entry(vote.user_id_hash.clone())
            .and_modify(|existing_vote| {
                if vote.timestamp > existing_vote.timestamp {
                    *existing_vote = vote.clone();
                }
            })
            .or_insert(vote);
    }

    // Count the votes
    let mut counts: HashMap<String, u32> = HashMap::new();
    for vote in latest_votes.values() {
        *counts.entry(vote.choice.clone()).or_insert(0) += 1;
    }

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount { choice, count })
        .collect();

    Ok(HttpResponse::Ok().json(vote_counts))
}

fn append_to_file<T: Serialize>(file_path: &str, value: &T) -> Result<(), Error> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("File error: {}", err))
        })?;

    // Serialize the value as JSON
    let json = serde_json::to_string(value).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("Serialization error: {}", err))
    })?;

    // Write the JSON string followed by a newline
    writeln!(file, "{}", json).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("File write error: {}", err))
    })?;

    Ok(())
}