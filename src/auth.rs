use actix_web::{http::header, HttpRequest, Error as ActixError, error};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::{env, fs};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub async fn validate_jwt(req: &HttpRequest) -> Result<Claims, ActixError> {
    // Extract the Authorization header
    let auth_header = req.headers().get(header::AUTHORIZATION);
    let token = match auth_header {
        Some(header_value) => {
            let auth_str = header_value.to_str().unwrap_or("");
            if auth_str.starts_with("Bearer ") {
                Some(auth_str.trim_start_matches("Bearer ").to_string())
            } else {
                None
            }
        }
        None => None,
    };

    let token = match token {
        Some(t) => t,
        None => {
            return Err(error::ErrorUnauthorized("Missing or invalid Authorization header"))
        }
    };

    // Load the public key from the file specified in JWT_PUBLIC_KEY_PATH
    let jwt_public_key_path =
        env::var("JWT_PUBLIC_KEY_PATH").expect("JWT_PUBLIC_KEY_PATH not set");
    let public_key_pem = fs::read_to_string(jwt_public_key_path).map_err(|err| {
        error::ErrorInternalServerError(format!("Failed to read public key file: {}", err))
    })?;
    let decoding_key = DecodingKey::from_ed_pem(public_key_pem.as_bytes()).map_err(|err| {
        error::ErrorInternalServerError(format!("Failed to create decoding key: {}", err))
    })?;

    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.validate_exp = true;

    // Decode and validate the JWT
    let token_data =
        decode::<Claims>(&token, &decoding_key, &validation).map_err(|err| {
            error::ErrorUnauthorized(format!("Invalid token: {}", err))
        })?;

    Ok(token_data.claims)
}