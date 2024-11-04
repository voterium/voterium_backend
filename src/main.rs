mod auth;
mod ch;
mod counting;
mod handlers;
mod models;
mod utils;
mod vote_logger;

use actix_cors::Cors;
use actix_web::middleware::from_fn;
use actix_web::{middleware::Logger, web, App, HttpServer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use clickhouse::Client;
use dotenv::dotenv;
use env_logger::Env;
use log::info;
use jsonwebtoken::DecodingKey;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use tokio;

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


    // Initialize ClickHouse client
    let clickhouse_host = env::var("CLICKHOUSE_HOST").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string());
    let clickhouse_database = env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "voting".to_string());
    let clickhouse_user = env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
    let clickhouse_password = env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| "".to_string());

    info!("Connecting to ClickHouse as {}", clickhouse_user);

    let clickhouse_client = Client::default()
        .with_url(&clickhouse_host)
        .with_database(&clickhouse_database)
        .with_user(&clickhouse_user)
        .with_password(&clickhouse_password);

    let count_ledger_filepath = "cl.csv";
    let vote_ledger_filepath = "vl.csv";

    // Create channel for vote logging
    // let (cl_sender, mut cl_receiver) = tokio::sync::mpsc::channel(10_000);
    // tokio::spawn(async move {
    //     vote_logger::write_lines_to_file(&count_ledger_filepath, cl_receiver).await.unwrap();
    // });

    // let (vl_sender, mut vl_receiver) = tokio::sync::mpsc::channel(10_000);
    // tokio::spawn(async move {
    //     vote_logger::write_lines_to_file(&vote_ledger_filepath, vl_receiver).await.unwrap();
    // });

    let (sender, mut receiver) = tokio::sync::mpsc::channel(10_000);
    tokio::spawn(async move {
        vote_logger::write_cl_vl(receiver).await.unwrap();
    });

    models::AppState {
        backend_salt,
        config,
        decoding_key,
        clickhouse_client,
        channel_sender: sender,
        // channel_sender_vl: vl_sender,
        // channel_sender_cl: cl_sender,
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
                    // .service(handlers::submit_vote)
                    .service(handlers::submit_vote2)
                    // .service(handlers::submit_vote3)
                    .service(handlers::get_results)
                    .service(handlers::get_config)
                    // .service(ch::vote2)
                    // .service(ch::get_results2)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}