// mod auth;
mod auth;
mod counting;
mod handlers;
mod models;

use actix_cors::Cors;
use actix_web::middleware::from_fn;
use actix_web::{middleware::Logger, web, App, HttpServer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use dotenv::dotenv;
use env_logger::Env;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::sync::Arc;

// use crate::auth::JwtMiddleware;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Get the backend salt from the environment variable
    let backend_salt = env::var("BACKEND_SALT").expect("BACKEND_SALT must be set");
    let backend_salt = URL_SAFE_NO_PAD
        .decode(&backend_salt)
        .expect("Invalid BACKEND_SALT; must be valid Base64");

    // Read voting_config.json
    let mut file = File::open("voting_config.json").expect("Failed to open voting_config.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read voting_config.json");
    let config: models::Config =
        serde_json::from_str(&contents).expect("Failed to parse voting_config.json");

    // Load the public key from the file specified in JWT_PUBLIC_KEY_PATH
    let jwt_public_key_path =
        env::var("JWT_PUBLIC_KEY_PATH").expect("JWT_PUBLIC_KEY_PATH not set");
    let public_key_pem = fs::read_to_string(jwt_public_key_path)
        .expect("Failed to read public key file");
    let decoding_key = jsonwebtoken::DecodingKey::from_ed_pem(public_key_pem.as_bytes())
        .expect("Failed to create decoding key");
    let decoding_key = Arc::new(decoding_key);

    // // Create JWT middleware
    // let jwt_middleware = JwtMiddleware {
    //     decoding_key: decoding_key.clone(),
    // };

    let state = models::AppState {
        backend_salt,
        config,
    };

    // Start the HTTP server
    HttpServer::new(move || {
        let cors = Cors::permissive(); // Create a permissive CORS policy

        App::new()
            .wrap(Logger::default())
            .wrap(cors) // Apply the CORS middleware
            .wrap(from_fn(auth::jwt_middleware))
            .app_data(web::Data::new(state.clone()))
            .service(
                web::scope("/voting")
                    .service(handlers::vote).wrap(from_fn(auth::jwt_middleware))
                    .service(handlers::get_results)
                    .service(handlers::get_config),
            )
    })
    .bind("127.0.0.1:8080")? // Bind to localhost on port 8080
    .run()
    .await
}