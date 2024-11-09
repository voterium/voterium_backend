mod auth;
mod counting;
mod errors;
mod handlers;
mod models;
mod utils;
mod vote_logger;

use actix_cors::Cors;
use actix_web::middleware::from_fn;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use env_logger::Env;
use models::AppState;
use utils::{
    load_backend_salt, load_public_key, load_voting_config, spawn_cache_worker,
    spawn_logging_worker,
};

pub use crate::errors::{AppError, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let config = load_voting_config().await;
    let state = AppState {
        backend_salt: load_backend_salt().await,
        decoding_key: load_public_key().await,
        logging_channel_sender: spawn_logging_worker().await,
        cache_channel_sender: spawn_cache_worker(config.choices.clone()).await,
        config,
    };

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(Logger::default())
            .wrap(from_fn(auth::jwt_middleware))
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .service(
                web::scope("/voting")
                    .service(handlers::submit_vote)
                    .service(handlers::get_results)
                    .service(handlers::get_config),
            )
    })
    .workers(1)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
