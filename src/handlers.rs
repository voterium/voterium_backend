use crate::counting::{count_votes, load_cl};
use crate::errors::{AppError, Result};
use crate::models::{AppState, Claims, Vote};
use crate::utils::gen_random_b64_string;
use crate::utils::hash_user_id;
use crate::vote_logger::{CountsCacheMsg, VLCLMessage};
use actix_web::HttpMessage;
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use chrono::Utc;
use log::info;
use std::time::Instant;
// use tokio::sync::oneshot;


#[post("/vote")]
pub async fn submit_vote(
    app_state: web::Data<AppState>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_vote = Instant::now();
    let timestamp = Utc::now().timestamp_millis();

    // Store extensions in a variable to extend its lifetime
    let extensions = req.extensions();
    let claims = extensions.get::<Claims>().ok_or(AppError::InternalError {
        title: "Claims not found".into(),
        message: "Could not find claims in req.extensions()".into(),
    })?;

    // Hash the user_id to generate the vote_id receipt
    let start_hash = Instant::now();
    let user_id = &claims.sub;
    let user_salt = &claims.salt;
    let backend_salt = &app_state.backend_salt;
    let vote_id = gen_random_b64_string(12);

    let user_id_hash = hash_user_id(user_id, user_salt, backend_salt)?;
    let hash_duration = start_hash.elapsed();

    // Verify that the choice is valid
    if !app_state
        .config
        .choices
        .iter()
        .any(|c| c.key == vote.choice)
    {
        let message = format!(
            "Choice must be one of {:?}. Received: {:?}",
            app_state.config.choices, vote.choice
        );
        return Err(AppError::BadRequest {
            title: "Invalid choice".to_string(),
            message,
        });
    };

    let start_vl_cl = Instant::now();
    let sender = &app_state.logging_channel_sender;
    // let (resp_tx, resp_rx) = oneshot::channel::<bool>();
    let cl_line = format!("{},{},{}\n", user_id_hash, timestamp, vote.choice).into_bytes();
    let msg = VLCLMessage {
        vl_data: format!("{},{}\n", vote_id, vote.choice).into_bytes(),
        cl_data: cl_line.clone(),
        // resp: resp_tx,
    };

    sender.send(msg).await?;
    // resp_rx.await.expect("Error receiving response from channel");

    let vl_cl_duration = start_vl_cl.elapsed();

    let cache_sender = &app_state.cache_channel_sender;
    cache_sender
        .send(CountsCacheMsg::Vote { cl_line })
        .await?;


    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, write VL CL {:?}, /vote: {:?}",
        hash_duration, vl_cl_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}


#[get("/results")]
pub async fn get_results(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    // let allowed_choices = &app_state.config.choices;

    // let cl_data = load_data("cl.csv")?;
    // let mut vote_counts = count_votes(&cl_data, allowed_choices)?;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app_state.cache_channel_sender.send(CountsCacheMsg::GetCounts { resp: tx }).await?;
    let mut vote_counts = rx.await?;
    vote_counts.sort_by(|a, b| a.choice.cmp(&b.choice));

    Ok(HttpResponse::Ok().json(vote_counts))
}


#[get("/config")]
pub async fn get_config(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let config = &app_state.config;
    Ok(HttpResponse::Ok().json(config))
}
