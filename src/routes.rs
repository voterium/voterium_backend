use crate::auth::validate_jwt;
use crate::models::{Vote, VoteCount};
use actix_web::{web, HttpRequest, HttpResponse, Error};
use chrono::Utc;
use sqlx::SqlitePool;

pub async fn vote(
    pool: web::Data<SqlitePool>,
    vote: web::Json<Vote>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    // Validate JWT and extract claims
    let claims = validate_jwt(&req).await?;

    let user_id = claims.sub;

    // Insert the vote into the database
    let timestamp = Utc::now().timestamp() as i64;

    sqlx::query(
        "INSERT INTO votes (user_id, choice, timestamp) VALUES (?, ?, ?)",
    )
    .bind(user_id)
    .bind(&vote.choice)
    .bind(timestamp)
    .execute(pool.get_ref())
    .await
    .map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", err))
    })?;

    Ok(HttpResponse::Ok().body("Vote recorded"))
}

pub async fn results(
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, Error> {
    // Query to get the counts of votes, considering only the most recent vote per user
    let vote_counts = sqlx::query_as::<_, VoteCount>(
        r#"
        SELECT choice, COUNT(*) as count FROM (
            SELECT user_id, choice FROM votes
            WHERE (user_id, timestamp) IN (
                SELECT user_id, MAX(timestamp) FROM votes GROUP BY user_id
            )
        ) GROUP BY choice;
        "#
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!("Database error: {}", err))
    })?;

    Ok(HttpResponse::Ok().json(vote_counts))
}
