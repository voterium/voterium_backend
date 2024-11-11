mod auth;
mod counting;
mod errors;
mod handlers;
mod models;
mod utils;
mod workers;

use actix_cors::Cors;
use actix_web::middleware::from_fn;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use env_logger::Env;
use models::AppState;
use utils::{
    load_backend_salt, load_cl_filepath, load_public_key, load_vl_filepath, load_voting_config,
    spawn_count_worker, spawn_ledger_worker,
};

pub use crate::errors::{AppError, Result};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let config = load_voting_config().await;
    let cl_filepath = load_cl_filepath().await;
    let vl_filepath = load_vl_filepath().await;
    let state = AppState {
        backend_salt: load_backend_salt().await,
        decoding_key: load_public_key().await,
        ledger_channel_sender: spawn_ledger_worker(&cl_filepath, &vl_filepath).await,
        count_channel_sender: spawn_count_worker(config.choices.clone(), &cl_filepath).await,
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
