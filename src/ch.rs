use crate::utils::hash_user_id;
use crate::{
    utils::gen_random_b64_string,
    models::{AppState, CLVote, Claims, VLVote, Vote, VoteCount},
};
use actix_web::{get, post, web, Error, HttpMessage, HttpRequest, HttpResponse};
use chrono::Utc;
use clickhouse::Row;
use log::{error, info};
use serde::{Serialize, Deserialize};
use std::time::Instant;

// Existing imports...

// Define a struct for ClickHouse vote insertion
#[derive(Row, Serialize)]
struct ClickHouseVote {
    vote_id: String,
    user_id_hash: String,
    timestamp: i64,
    choice: String,
}

// Define a struct for ClickHouse vote counts
#[derive(Serialize, Deserialize, Row)]
struct ClickHouseVoteCount {
    choice: String,
    count: u64,
}

#[post("/vote")]
pub async fn vote2(
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
    if !app_state
        .config
        .choices
        .iter()
        .any(|c| c.key == vote.choice)
    {
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Invalid choice: {}",
            vote.choice
        )));
    }

    // Get current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    // Prepare the ClickHouse vote
    let ch_vote = ClickHouseVote {
        vote_id: vote_id.clone(),
        user_id_hash: user_id_hash.clone(),
        timestamp,
        choice: vote.choice.clone(),
    };

    // Insert into ClickHouse
    let insert_start = Instant::now();

    if let Ok(mut insert) = app_state.clickhouse_client.insert("voting.votes") {
        match insert.write(&ch_vote).await {
            Ok(_) => {
                match insert.end().await {
                    Ok(_) => {
                        info!("Inserted vote into ClickHouse");
                    },
                    Err(e) => {
                        error!("ClickHouse insert error: {}", e);
                        return Err(actix_web::error::ErrorInternalServerError(format!(
                            "ClickHouse insert error: {}",
                            e
                        )));
                    }
                }
            }
            Err(e) => {
                error!("ClickHouse write error: {}", e);
                return Err(actix_web::error::ErrorInternalServerError(format!(
                    "ClickHouse write error: {}",
                    e
                )));
            }
        }
    };

    let insert_duration = insert_start.elapsed();

    // if let Err(e) = insert_result {
    //     return Err(actix_web::error::ErrorInternalServerError(format!(
    //         "ClickHouse insert error: {}",
    //         e
    //     )));
    // }

    // Log total time for /vote2 function
    let total_duration = start_vote.elapsed();
    info!(
        "hash user_id: {:?}, insert ClickHouse {:?}, /vote2: {:?}",
        hash_duration, insert_duration, total_duration
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({ "vote_id": vote_id })))
}

#[get("/results")]
pub async fn get_results2(app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let start_query = Instant::now();

    // Construct the SQL query to aggregate votes
    let query = "
        SELECT choice, count(*) as count
        FROM voting.votes
        GROUP BY choice
        ORDER BY choice
    ";

    // Execute the query
    let vote_counts: Vec<ClickHouseVoteCount> = app_state
        .clickhouse_client
        .query(query)
        .fetch_all::<ClickHouseVoteCount>()
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("ClickHouse query error: {}", e))
        })?;

    let query_duration = start_query.elapsed();
    info!(
        "ClickHouse query {:?} executed in {:?}",
        query, query_duration
    );

    Ok(HttpResponse::Ok().json(vote_counts))
}
