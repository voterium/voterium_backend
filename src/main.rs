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
use jsonwebtoken::DecodingKey;
use std::env;
use std::fs::{self, File};
use std::io::Read;


async fn load_app_state() -> models::AppState {
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

    // Load the public key
    let jwt_public_key_path = env::var("JWT_PUBLIC_KEY_PATH").expect("JWT_PUBLIC_KEY_PATH not set");
    let public_key_pem = fs::read_to_string(jwt_public_key_path).expect("Failed to read public key");
    let decoding_key = DecodingKey::from_ed_pem(public_key_pem.as_bytes()).expect("Failed to create DecodingKey from public key");

    models::AppState {
        backend_salt,
        config,
        decoding_key,
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let state = load_app_state().await;

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(Logger::default())
            .wrap(from_fn(auth::jwt_middleware))
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .service(
                web::scope("/voting")
                    .service(handlers::vote)
                    .service(handlers::get_results)
                    .service(handlers::get_config),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}