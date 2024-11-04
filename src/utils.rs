use actix_web::Error;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use blake2::{digest::consts::U12, Blake2b, Digest};
use rand::{rngs::OsRng, RngCore};

type Blake2b96 = Blake2b<U12>; // 96 bytes = 12 * 8 bits

pub(crate) fn hash_user_id(
    user_id: &str,
    user_salt: &str,
    backend_salt_bytes: &[u8],
) -> Result<String, Error> {
    // Combine user_id with user_salt and backend_salt
    let mut hasher = Blake2b96::new();

    // Convert salts from Base64 if necessary
    let user_salt_bytes = URL_SAFE_NO_PAD.decode(user_salt).map_err(|err| {
        actix_web::error::ErrorInternalServerError(format!(
            "User salt decode error: {}, user_salt {:?}",
            err, user_salt
        ))
    })?;

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
