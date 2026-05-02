use serde::Deserialize;
use crate::utils::error::ApiError;
#[derive(Deserialize)]

pub struct GoogleTokenInfo {
    pub sub: String,          // Google User ID unique
    pub email: String,
    pub name: String,
    pub aud: String,          // Audience (Client ID)
}

pub async fn verify_google_token(id_token: &str) -> Result<GoogleTokenInfo, ApiError> {
    let client = reqwest::Client::new();
    let url = format!("https://oauth2.googleapis.com/tokeninfo?id_token={}", id_token);

    let response = client.get(url)
        .send()
        .await
        .map_err(|_| ApiError::internal("Failed connect to Google Auth"))?;

    if !response.status().is_success() {
        return Err(ApiError::unauthorized("Google Token invalid"));
    }

    let info: GoogleTokenInfo = response.json()
        .await
        .map_err(|_| ApiError::internal("Failed to parse Google Token"))?;

    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| ApiError::internal("GOOGLE_CLIENT_ID is not configured"))?;
        
    if info.aud != client_id {
        return Err(ApiError::unauthorized("Invalid token audience"));
    }

    return Ok(info);
}
