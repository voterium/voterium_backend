use std::{
    env,
    fs::{self, File},
    io::Read,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use blake2::{digest::consts::U12, Blake2b, Digest};
use jsonwebtoken::DecodingKey;
use rand::{rngs::OsRng, RngCore};

use crate::{
    errors::Result,
    models::{Choice, Config, CountWorkerMsg, LedgerWorkerMsg},
    workers::{run_counts_worker, run_ledger_worker},
};

type Blake2b96 = Blake2b<U12>; // 96 bytes = 12 * 8 bits

pub fn hash_user_id(user_id: &str, user_salt: &str, backend_salt_bytes: &[u8]) -> Result<String> {
    // Combine user_id with user_salt and backend_salt.
    // user_salt and backend_salt should be 8 bytes each.

    let mut hasher = Blake2b96::new();

    let user_salt = URL_SAFE_NO_PAD.decode(user_salt)?;

    hasher.update(user_id.as_bytes());
    hasher.update(&user_salt);
    hasher.update(&backend_salt_bytes);

    let result = hasher.finalize();

    let user_id_hash = URL_SAFE_NO_PAD.encode(&result);

    Ok(user_id_hash)
}

pub fn gen_random_b64_string(length: usize) -> String {
    let mut random_bytes = vec![0u8; length];
    OsRng.fill_bytes(&mut random_bytes);
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

pub fn load_voting_config(filepath: &str) -> Config {
    let mut file = File::open(filepath).expect("Failed to open voting_config.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read voting_config.json");
    let config: Config =
        serde_json::from_str(&contents).expect("Failed to parse voting_config.json");

    validate_unique_choice_keys(&config.choices);
    config
}

pub fn validate_unique_choice_keys(choices: &[Choice]) -> () {
    let mut seen_keys = std::collections::HashSet::new();
    for choice in choices {
        if choice.key.len() < 1 {
            panic!("Choice key must not be empty");
        }

        if !seen_keys.insert(choice.key_u8()) {
            panic!("First character of choice key must be unique");
        }
    }
}

pub fn load_backend_salt() -> Vec<u8> {
    // Get the backend salt from the environment variable
    let backend_salt = env::var("BACKEND_SALT").expect("BACKEND_SALT must be set");
    let backend_salt = URL_SAFE_NO_PAD
        .decode(&backend_salt)
        .expect("Invalid BACKEND_SALT; must be valid Base64");

    assert!(backend_salt.len() == 8, "BACKEND_SALT must be 8 bytes long");

    backend_salt
}

pub fn load_public_key() -> DecodingKey {
    let jwt_public_key_path = env::var("JWT_PUBLIC_KEY_PATH").unwrap_or("key.pub".to_string());
    let public_key_pem =
        fs::read_to_string(jwt_public_key_path).expect("Failed to read public key");
    let decoding_key = DecodingKey::from_ed_pem(public_key_pem.as_bytes())
        .expect("Failed to create DecodingKey from public key");
    decoding_key
}

pub fn load_cl_filepath() -> String {
    let cl_filepath = env::var("CL_FILEPATH").unwrap_or("cl.csv".to_string());
    cl_filepath
}

pub fn load_vl_filepath() -> String {
    let vl_filepath = env::var("VL_FILEPATH").unwrap_or("vl.csv".to_string());
    vl_filepath
}

pub fn load_config_filepath() -> String {
    let config_filepath = env::var("CONFIG_FILEPATH").unwrap_or("voting_config.json".to_string());
    config_filepath
}

pub async fn spawn_ledger_worker(
    cl_filepath: &str,
    vl_filepath: &str,
) -> tokio::sync::mpsc::Sender<LedgerWorkerMsg> {
    let (tx, rx) = tokio::sync::mpsc::channel(10_000);
    let cl_filepath = cl_filepath.to_owned();
    let vl_filepath = vl_filepath.to_owned();
    tokio::spawn(async move {
        run_ledger_worker(rx, &cl_filepath, &vl_filepath)
            .await
            .expect("Ledger worker failed");
    });
    tx
}

pub async fn spawn_count_worker(
    choices: Vec<Choice>,
    cl_filepath: &str,
) -> tokio::sync::mpsc::Sender<CountWorkerMsg> {
    let (tx, rx) = tokio::sync::mpsc::channel(10_000);
    let cl_filepath = cl_filepath.to_owned();
    tokio::spawn(async move {
        run_counts_worker(rx, &cl_filepath, &choices)
            .await
            .expect("Count worker failed");
    });
    tx
}
