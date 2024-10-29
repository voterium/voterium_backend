mod auth;
mod models;
mod routes;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;
use env_logger::Env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Get the database URL from the environment variable
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool to the SQLite database
    let pool = SqlitePool::connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Ensure the database schema is created
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS votes (
            id INTEGER PRIMARY KEY,
            user_id TEXT NOT NULL,
            choice TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create votes table");

    let data = web::Data::new(pool);
    let state = routes::AppState {
        backend_salt: env::var("BACKEND_SALT").expect("BACKEND_SALT must be set"),
    };

    // Start the HTTP server
    HttpServer::new(move || {
        let cors = Cors::permissive(); // Create a permissive CORS policy

        App::new()
            .wrap(Logger::default())
            .wrap(cors) // Apply the CORS middleware
            .app_data(data.clone())
            .app_data(web::Data::new(state.clone()))
            .service(
                web::resource("/vote")
                    .route(web::post().to(routes::vote))
            )
            .service(
                web::resource("/results")
                    .route(web::get().to(routes::results))
            )
    })
    .bind("127.0.0.1:8080")? // Bind to localhost on port 8080
    .run()
    .await
}
