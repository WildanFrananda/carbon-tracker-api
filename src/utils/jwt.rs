use jsonwebtoken::errors::Error;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

pub fn generate_access_token(user_id: Uuid) -> Result<String, Error> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(1))
        .expect("Valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn generate_refresh_token() -> String {
    return Uuid::new_v4().to_string();
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    return hex::encode(result);
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Claims {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = request.headers().get_one("Authorization");

        if let Some(auth_str) = auth_header {
            if auth_str.starts_with("Bearer ") {
                let token = auth_str.trim_start_matches("Bearer ");
                let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");

                match decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::default(),
                ) {
                    Ok(token_data) => return Outcome::Success(token_data.claims),
                    Err(_) => return Outcome::Error((Status::Unauthorized, ())),
                }
            }
        }

        return Outcome::Error((Status::Unauthorized, ()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_generate_and_decode_access_token() {
        env::set_var("JWT_SECRET", "supersecretkey");
        let user_id = Uuid::new_v4();
        
        // Generate Token
        let token_res = generate_access_token(user_id);
        assert!(token_res.is_ok(), "Token generation should succeed");
        
        let token = token_res.unwrap();
        
        // Decode Token
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("supersecretkey".as_bytes()),
            &Validation::default(),
        );
        
        assert!(decoded.is_ok(), "Token decoding should succeed");
        let claims = decoded.unwrap().claims;
        assert_eq!(claims.sub, user_id, "User ID should match");
        assert!(claims.exp > 0, "Expiration should be set");
    }

    #[test]
    fn test_hash_refresh_token() {
        let raw_token = "my-refresh-token";
        let hashed = hash_refresh_token(raw_token);
        
        // Ensure it's not the same as raw token
        assert_ne!(raw_token, hashed);
        // Sha256 hex is 64 chars
        assert_eq!(hashed.len(), 64);
    }
}
