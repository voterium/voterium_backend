use voterium_backend::{auth, handlers, models, utils};

use actix_cors::Cors;
use actix_web::middleware::from_fn;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use env_logger::Env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let cl_filepath = utils::load_cl_filepath();
    let vl_filepath = utils::load_vl_filepath();
    let config_filepath = utils::load_config_filepath();

    let config = utils::load_voting_config(&config_filepath);
    
    let state = models::AppState {
        backend_salt: utils::load_backend_salt(),
        decoding_key: utils::load_public_key(),
        ledger_channel_sender: utils::spawn_ledger_worker(&cl_filepath, &vl_filepath).await,
        count_channel_sender: utils::spawn_count_worker(config.choices.clone(), &cl_filepath).await,
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
