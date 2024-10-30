use crate::auth::{gen_random_b64_string, validate_jwt};
use crate::models::{AppState, Vote, VoteCount, CLVote, VLVote};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use blake2::{Blake2b, Digest, digest::consts::U12};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

type Blake2b96 = Blake2b<U12>;  // 96 bytes = 12 * 8 bits

fn hash_user_id(
    user_id: &str,
    user_salt: &str,
    backend_salt: &Vec<u8>,
) -> Result<String, Error> {
    // Combine user_id with user_salt and backend_salt
    let mut hasher = Blake2b96::new();

    // Convert salts from Base64 if necessary (assuming they are base64-encoded)
    let user_salt_bytes = URL_SAFE_NO_PAD.decode(user_salt).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("User salt decode error: {}, user_salt {:?}", err, user_salt))
    })?;
    let backend_salt_bytes = backend_salt;

    // Input data: user_id || user_salt || backend_salt
    println!("user_id: {}, user_salt: {:?}, backend_salt: {:?}", user_id, user_salt_bytes, backend_salt_bytes);
    hasher.update(user_id.as_bytes());
    hasher.update(&user_salt_bytes);
    hasher.update(&backend_salt_bytes);

    // Finalize the hash
    let result = hasher.finalize();

    // Convert hash to hexadecimal string
    // let user_id_hash = hex::encode(result);
    let user_id_hash = URL_SAFE_NO_PAD.encode(&result);

    Ok(user_id_hash)
}

use std::time::Instant;

pub async fn vote(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let start_vote = Instant::now();

    // Validate JWT and extract claims
    let claims = validate_jwt(&req).await?;

    let user_id = claims.sub;
    let user_salt = claims.salt;
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    // Hash the user_id to generate the vote_id receipt
    let start_hash = Instant::now();
    let user_id_hash = hash_user_id(&user_id, &user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();
    println!("Time to hash user_id: {:?}", hash_duration);

    // Get current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    // Measure time to append to Public Vote Verification Ledger (VL)
    let start_vl = Instant::now();
    let vl_vote = VLVote {
        vote_id: vote_id.clone(),
        choice: vote.choice.clone(),
    };
    append_to_vl("vl.csv", &vl_vote)?;
    let vl_duration = start_vl.elapsed();
    println!("Time to write to VL: {:?}", vl_duration);

    // Measure time to append to Public Vote Count Ledger (CL)
    let start_cl = Instant::now();
    let cl_vote = CLVote {
        user_id_hash,
        timestamp,
        choice: vote.choice.clone(),
    };
    append_to_cl("cl.csv", &cl_vote)?;
    let cl_duration = start_cl.elapsed();
    println!("Time to write to CL: {:?}", cl_duration);

    // Log total time for /vote function
    let total_duration = start_vote.elapsed();
    println!("hash user_id: {:?}, write VL {:?}, write CL {:?}, /vote: {:?}", hash_duration, vl_duration, cl_duration, total_duration);

    Ok(HttpResponse::Ok().body("Vote recorded"))
}

pub async fn results(_req: HttpRequest) -> Result<HttpResponse, Error> {
    // Read the CL file
    let file = File::open("cl.csv").map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("File error: {}", err))
    })?;

    let reader = BufReader::new(file);
    let mut votes: Vec<CLVote> = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("File read error: {}", err))
        })?;

        let parts: Vec<&str> = line.trim().split(',').collect();
        if parts.len() != 3 {
            continue; // Skip malformed lines
        }

        let vote = CLVote {
            user_id_hash: parts[0].to_string(),
            timestamp: parts[1].parse::<i64>().map_err(|err| {
                actix_web::error::ErrorInternalServerError(format!("Parse error: {}", err))
            })?,
            choice: parts[2].to_string(),
        };

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

fn append_to_cl(file_path: &str, cl_vote: &CLVote) -> Result<(), Error> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("File error: {}", err))
        })?;

    // Format the line
    let line = format!("{},{},{}\n", cl_vote.user_id_hash, cl_vote.timestamp, cl_vote.choice);

    // Write the line to the file
    file.write_all(line.as_bytes()).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("File write error: {}", err))
    })?;

    Ok(())
}

fn append_to_vl(file_path: &str, vl_vote: &VLVote) -> Result<(), Error> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| {
            actix_web::error::ErrorInternalServerError(format!("File error: {}", err))
        })?;

    // Format the line
    let line = format!("{},{}\n", vl_vote.vote_id, vl_vote.choice);

    // Write the line to the file
    file.write_all(line.as_bytes()).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("File write error: {}", err))
    })?;

    Ok(())
}