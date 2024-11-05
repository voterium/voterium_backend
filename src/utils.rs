use std::{fs::File, io::Read};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use blake2::{digest::consts::U12, Blake2b, Digest};
use rand::{rngs::OsRng, RngCore};

use crate::{errors::Result, models::Config};

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

pub fn load_voting_config() -> Config {
    let mut file = File::open("voting_config.json").expect("Failed to open voting_config.json");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read voting_config.json");
    let config: Config =
        serde_json::from_str(&contents).expect("Failed to parse voting_config.json");
        config
}