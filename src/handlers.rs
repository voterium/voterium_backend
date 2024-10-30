use crate::auth::{gen_random_b64_string, validate_jwt};
use crate::models::{AppState, Vote, CLVote, VLVote};
use crate::counting::{count_votes_1, count_votes_2, count_votes_3, count_votes_4, count_votes_5, count_votes_6, count_votes_7, count_votes_8, count_votes_9};
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use blake2::{Blake2b, Digest, digest::consts::U12};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use log::info;

type Blake2b96 = Blake2b<U12>;  // 96 bytes = 12 * 8 bits

fn hash_user_id(
    user_id: &str,
    user_salt: &str,
    backend_salt_bytes: &[u8],
) -> Result<String, Error> {
    // Combine user_id with user_salt and backend_salt
    let mut hasher = Blake2b96::new();

    // Convert salts from Base64 if necessary (assuming they are base64-encoded)
    let user_salt_bytes = URL_SAFE_NO_PAD.decode(user_salt).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("User salt decode error: {}, user_salt {:?}", err, user_salt))
    })?;

    // Input data: user_id || user_salt || backend_salt
    hasher.update(user_id.as_bytes());
    hasher.update(&user_salt_bytes);
    hasher.update(&backend_salt_bytes);
    let result = hasher.finalize();

    // Convert hash to hexadecimal string
    let user_id_hash = URL_SAFE_NO_PAD.encode(&result);

    Ok(user_id_hash)
}

use std::time::Instant;

#[post("/vote")]
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

    // Measure time to append to Public Vote Count Ledger (CL)
    let start_cl = Instant::now();
    let cl_vote = CLVote {
        user_id_hash,
        timestamp,
        choice: vote.choice.clone(),
    };
    append_to_cl("cl.csv", &cl_vote)?;
    let cl_duration = start_cl.elapsed();

    // Log total time for /vote function
    let total_duration = start_vote.elapsed();
    info!("hash user_id: {:?}, write VL {:?}, write CL {:?}, /vote: {:?}", hash_duration, vl_duration, cl_duration, total_duration);

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}

// Updated results handler
#[get("/results")]
pub async fn results(_req: HttpRequest) -> Result<HttpResponse, Error> {
    let vote_counts = count_votes_6().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error counting votes: {}", e))
    })?;
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