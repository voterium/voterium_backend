use std::{env, fs::{self, File}, io::Read};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use blake2::{digest::consts::U12, Blake2b, Digest};
use jsonwebtoken::DecodingKey;
use rand::{rngs::OsRng, RngCore};

use crate::{errors::Result, models::Config, vote_logger};

type Blake2b96 = Blake2b<U12>; // 96 bytes = 12 * 8 bits

pub fn hash_user_id(
    user_id: &str,
    user_salt: &str,
    backend_salt_bytes: &[u8],
) -> Result<String> {
    // Combine user_id with user_salt and backend_salt
    let mut hasher = Blake2b96::new();

    // Convert salts from Base64 if necessary
    let user_salt_bytes = URL_SAFE_NO_PAD.decode(user_salt)?;

    hasher.update(user_id.as_bytes());
    hasher.update(&user_salt_bytes);
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

pub async fn load_voting_config() -> Config {
    let mut file = File::open("voting_config.json").expect("Failed to open voting_config.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read voting_config.json");
    let config: Config =
        serde_json::from_str(&contents).expect("Failed to parse voting_config.json");
        config
}


pub async fn load_backend_salt() -> Vec<u8> {
    // Get the backend salt from the environment variable
    let backend_salt = env::var("BACKEND_SALT").expect("BACKEND_SALT must be set");
    let backend_salt = URL_SAFE_NO_PAD
        .decode(&backend_salt)
        .expect("Invalid BACKEND_SALT; must be valid Base64");

    backend_salt
}

pub async fn load_public_key() -> DecodingKey {
    let jwt_public_key_path = env::var("JWT_PUBLIC_KEY_PATH").expect("JWT_PUBLIC_KEY_PATH not set");
    let public_key_pem = fs::read_to_string(jwt_public_key_path).expect("Failed to read public key");
    let decoding_key = DecodingKey::from_ed_pem(public_key_pem.as_bytes()).expect("Failed to create DecodingKey from public key");
    decoding_key
}

pub async fn spawn_logging_worker() -> tokio::sync::mpsc::Sender<vote_logger::VLCLMessage> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(10_000);
    tokio::spawn(async move {
        vote_logger::write_cl_vl(receiver).await.unwrap();
    });
    sender
}