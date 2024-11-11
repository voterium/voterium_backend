use crate::counting::{count_votes, load_cl};
use crate::errors::{AppError, Result};
use crate::models::{AppState, Ballot, Choice, Claims, CountWorkerBallot, CountWorkerMsg, LedgerWorkerMsg, Vote};
use crate::utils::gen_random_b64_string;
use crate::utils::hash_user_id;
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

    verify_valid_choice(&vote, &app_state.config.choices)?;

    let start_send_msgs = Instant::now();
    let ballot = Ballot{
        vote_id: vote_id.clone(),
        user_id_hash,
        timestamp,
        choice: vote.choice.clone(), 
    };

    let ledger_sender = &app_state.ledger_channel_sender;
    let msg = LedgerWorkerMsg::from(&ballot);
    ledger_sender.send(msg).await?;

    let count_sender = &app_state.count_channel_sender;
    let msg = CountWorkerMsg::Vote { ballot: CountWorkerBallot::from(&ballot) };
    count_sender.send(msg).await?;

    let send_msgs_duration = start_send_msgs.elapsed();

    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, send VL CL messages {:?}, /vote: {:?}",
        hash_duration, send_msgs_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}


#[get("/results")]
pub async fn get_results(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app_state.count_channel_sender.send(CountWorkerMsg::GetCounts { resp: tx }).await?;
    let mut vote_counts = rx.await?;
    vote_counts.sort_by(|a, b| a.choice.cmp(&b.choice));

    Ok(HttpResponse::Ok().json(vote_counts))
}


#[get("/config")]
pub async fn get_config(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let config = &app_state.config;
    Ok(HttpResponse::Ok().json(config))
}


fn verify_valid_choice(vote: &Vote, choices: &[Choice]) -> Result<()> {
    if !choices
        .iter()
        .any(|c| c.key == vote.choice)
    {
        let message = format!(
            "Choice must be one of {:?}. Received: {:?}",
            choices, vote.choice
        );
        return Err(AppError::BadRequest {
            title: "Invalid choice".to_string(),
            message,
        });
    };
    Ok(())
}