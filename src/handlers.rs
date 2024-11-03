use crate::auth::gen_random_b64_string;
use crate::counting::count_votes;
use crate::models::{AppState, CLVote, Claims, VLVote, Vote};
use crate::utils::hash_user_id;
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use chrono::Utc;
use log::info;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

// Import HttpMessage to access extensions
use actix_web::HttpMessage;




#[post("/vote")]
pub async fn vote(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let start_vote = Instant::now();

    // Store extensions in a variable to extend its lifetime
    let extensions = req.extensions();
    let claims = extensions.get::<Claims>().ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("Claims not found in request extensions")
    })?;

    let user_id = claims.sub.clone();
    let user_salt = claims.salt.clone();
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    // Hash the user_id to generate the vote_id receipt
    let start_hash = Instant::now();
    let user_id_hash = hash_user_id(&user_id, &user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();

    // Verify that the choice is valid
    if !app_state.config.choices.iter().any(|c| c.key == vote.choice) {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Invalid choice: {}",
            vote.choice
        )));
    }

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
    info!(
        "hash user_id: {:?}, write VL {:?}, write CL {:?}, /vote: {:?}",
        hash_duration, vl_duration, cl_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}

// Updated results handler
#[get("/results")]
pub async fn get_results(
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let allowed_choices = &app_state.config.choices;

    let mut vote_counts = count_votes(allowed_choices).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error counting votes: {}", e))
    })?;

    // Sort vote_counts by choice
    vote_counts.sort_by(|a, b| a.choice.cmp(&b.choice));

    Ok(HttpResponse::Ok().json(vote_counts))
}

#[get("/config")]
pub async fn get_config(app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let config = &app_state.config;
    Ok(HttpResponse::Ok().json(config))
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
    let line = format!(
        "{},{},{}\n",
        cl_vote.user_id_hash, cl_vote.timestamp, cl_vote.choice
    );

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
