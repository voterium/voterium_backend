use crate::utils::gen_random_b64_string;
use crate::counting::count_votes;
use crate::models::{AppState, CLVote, Claims, VLVote, Vote};
use crate::utils::hash_user_id;
use crate::vote_logger::VLCLMessage;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse}; // Error
use chrono::Utc;
use log::info;
use tokio::sync::oneshot;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

// Import HttpMessage to access extensions
use actix_web::HttpMessage;

use crate::{Result, AppError};

#[post("/vote")]
pub async fn submit_vote(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_vote = Instant::now();

    // Store extensions in a variable to extend its lifetime
    let extensions = &req.extensions();
    let claims = extensions.get::<Claims>().ok_or(
        AppError::InternalError{ 
            title: "Claims not found".into(), 
            message: "Could not find claims in req.extensions()".into() }
    )?;
    
    let start_hash = Instant::now();
    let user_id = &claims.sub;
    let user_salt = &claims.salt;
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    let user_id_hash = hash_user_id(&user_id, &user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();

    // Verify that the choice is valid
    if !app_state.config.choices.iter().any(|c| c.key == vote.choice) {
        let message = format!("Choice must be one of {:?}. Received: {:?}", app_state.config.choices, vote.choice);
        return Err(AppError::BadRequest{ title: "Invalid choice".to_string(), message });
    };

    // Get current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    // Measure time to append to Public Vote Verification Ledger (VL)
    let start_ledgers = Instant::now();
    let vl_vote = VLVote {
        vote_id: vote_id.clone(),
        choice: vote.choice.clone(),
    };
    append_to_vl("vl.csv", &vl_vote)?;
    
    // Measure time to append to Public Vote Count Ledger (CL)
    let cl_vote = CLVote {
        user_id_hash,
        timestamp,
        choice: vote.choice.clone(),
    };
    append_to_cl("cl.csv", &cl_vote)?;
    let ledger_duration = start_ledgers.elapsed();

    // Log total time for /vote function
    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, write VL CL {:?}, /vote: {:?}",
        hash_duration, ledger_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}


#[post("/vote")]
pub async fn submit_vote1p1(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_vote = Instant::now();

    // Store extensions in a variable to extend its lifetime
    let extensions = req.extensions();
    let claims = extensions.get::<Claims>().ok_or(
        AppError::InternalError{ 
            title: "Claims not found".into(), 
            message: "Could not find claims in req.extensions()".into() }
    )?;
    
    // Hash the user_id to generate the vote_id receipt
    let start_hash = Instant::now();
    let user_id = claims.sub.clone();
    let user_salt = claims.salt.clone();
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    let user_id_hash = hash_user_id(&user_id, &user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();

    // Verify that the choice is valid
    if !app_state.config.choices.iter().any(|c| c.key == vote.choice) {
        let message = format!("Choice must be one of {:?}. Received: {:?}", app_state.config.choices, vote.choice);
        return Err(AppError::BadRequest{ title: "Invalid choice".to_string(), message });
    };

    // Get current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    let start_ledgers = Instant::now();
    append_to_vl2("vl.csv", &vote_id, &vote.choice)?;
    append_to_cl2("cl.csv", &user_id_hash, &timestamp, &vote.choice)?;
    let ledgers_duration = start_ledgers.elapsed();

    // Log total time for /vote function
    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, write VL CL {:?}, /vote: {:?}",
        hash_duration, ledgers_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}



#[post("/vote")]
pub async fn submit_vote2(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_vote = Instant::now();
    let timestamp = Utc::now().timestamp_millis();

    // Store extensions in a variable to extend its lifetime
    let extensions = req.extensions();
    let claims = extensions.get::<Claims>().ok_or(
        AppError::InternalError{ 
            title: "Claims not found".into(), 
            message: "Could not find claims in req.extensions()".into() }
    )?;
    
    // Hash the user_id to generate the vote_id receipt
    let start_hash = Instant::now();
    let user_id = &claims.sub;
    let user_salt = &claims.salt;
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    let user_id_hash = hash_user_id(user_id, user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();

    // Verify that the choice is valid
    if !app_state.config.choices.iter().any(|c| c.key == vote.choice) {
        let message = format!("Choice must be one of {:?}. Received: {:?}", app_state.config.choices, vote.choice);
        return Err(AppError::BadRequest{ title: "Invalid choice".to_string(), message });
    };

    let start_vl_cl = Instant::now();
    let sender = &app_state.channel_sender;
    let (resp_tx, resp_rx) = oneshot::channel::<bool>();
    let msg = VLCLMessage {
        vl_data: format!("{},{}\n", vote_id, vote.choice).into_bytes(),
        cl_data: format!("{},{},{}\n", user_id_hash, timestamp, vote.choice).into_bytes(),
        resp: resp_tx,
    };

    sender.send(msg).await?;

    // resp_rx.await.expect("Error receiving response from channel");
    let vl_cl_duration = start_vl_cl.elapsed();

    // Log total time for /vote function
    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, write VL CL {:?}, /vote: {:?}",
        hash_duration, vl_cl_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}



#[post("/vote")]
pub  async fn submit_vote3(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    println!("submit_vote3");
    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": "aaa" })))
}


// Updated results handler
#[get("/results")]
pub async fn get_results(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let allowed_choices = &app_state.config.choices;

    let mut vote_counts = count_votes(allowed_choices)?;

    // Sort vote_counts by choice
    vote_counts.sort_by(|a, b| a.choice.cmp(&b.choice));

    Ok(HttpResponse::Ok().json(vote_counts))
}

#[get("/config")]
pub async fn get_config(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let config = &app_state.config;
    Ok(HttpResponse::Ok().json(config))
}


fn append_to_cl2(file_path: &str, user_id_hash: &str, timestamp: &i64, choice: &str) -> Result<()> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // Format the line
    let line = format!("{},{},{}\n", user_id_hash, timestamp, choice);

    // Write the line to the file
    file.write_all(line.as_bytes())?;

    Ok(())
}


fn append_to_vl2(file_path: &str, vote_id: &str, choice: &str) -> Result<()> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // Format the line
    let line = format!("{},{}\n", vote_id, choice);

    // Write the line to the file
    file.write_all(line.as_bytes())?;

    Ok(())
}

fn append_to_cl(file_path: &str, cl_vote: &CLVote) -> Result<()> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // Format the line
    let line = format!(
        "{},{},{}\n",
        cl_vote.user_id_hash, cl_vote.timestamp, cl_vote.choice
    );

    // Write the line to the file
    file.write_all(line.as_bytes())?;

    Ok(())
}


fn append_to_vl(file_path: &str, vl_vote: &VLVote) -> Result<()> {
    // Open the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    // Format the line
    let line = format!("{},{}\n", vl_vote.vote_id, vl_vote.choice);

    // Write the line to the file
    file.write_all(line.as_bytes())?;

    Ok(())
}
