use crate::models::user::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest};
use crate::utils::error::ApiError;
use crate::utils::jwt::{generate_access_token, generate_refresh_token, hash_refresh_token};
use crate::DbPool;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use rocket::serde::json::{json, Json, Value};
use rocket::State;

#[post("/register", data = "<req>")]
pub async fn register(req: Json<RegisterRequest>, pool: &State<DbPool>) -> Result<Value, ApiError> {
    let email_exist = sqlx::query!("SELECT id FROM users WHERE email = $1", req.email)
        .fetch_optional(&pool.0)
        .await
        .map_err(|_| ApiError::internal("Database error while validating email"))?;

    if email_exist.is_some() {
        return Err(ApiError::bad_request("Email already exists"));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|_| ApiError::internal("Failed to hashing password"))?
        .to_string();

    let raw_refresh_token = generate_refresh_token();
    let hashed_refresh_token = hash_refresh_token(&raw_refresh_token);
    let refresh_expires_at = Utc::now() + Duration::days(7);

    let user = sqlx::query!(
        "INSERT INTO users (email, password_hash, display_name, refresh_token_hash, refresh_token_expires_at) VALUES ($1, $2, $3, $4, $5) RETURNING id, display_name",
        req.email, password_hash, req.display_name, hashed_refresh_token, refresh_expires_at
    )
    .fetch_one(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to register user"))?;

    let access_token = generate_access_token(user.id)
        .map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "User registered successfully",
        "data": AuthResponse {
            access_token,
            refresh_token: raw_refresh_token, // Kirim raw token ke client
            user_id: user.id,
            display_name: user.display_name,
        }
    }));
}

#[post("/login", data = "<req>")]
pub async fn login(req: Json<LoginRequest>, pool: &State<DbPool>) -> Result<Value, ApiError> {
    let user = sqlx::query!(
        "SELECT id, password_hash, display_name FROM users WHERE email = $1",
        req.email
    )
    .fetch_optional(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Database error while search the user"))?;

    let user = match user {
        Some(u) => u,
        None => return Err(ApiError::unauthorized("Email or password is wrong")),
    };

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| ApiError::internal("Invalid hash format in database"))?;

    if Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(ApiError::unauthorized("Email or password is wrong"));
    }

    let raw_refresh_token = generate_refresh_token();
    let hashed_refresh_token = hash_refresh_token(&raw_refresh_token);
    let refresh_expires_at = Utc::now() + Duration::days(7);

    sqlx::query!(
        "UPDATE users SET refresh_token_hash = $1, refresh_token_expires_at = $2 WHERE id = $3",
        hashed_refresh_token,
        refresh_expires_at,
        user.id
    )
    .execute(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to save login session"))?;

    let access_token = generate_access_token(user.id)
        .map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "Login successfully",
        "data": AuthResponse {
            access_token,
            refresh_token: raw_refresh_token,
            user_id: user.id,
            display_name: user.display_name,
        }
    }));
}

#[post("/refresh", data = "<req>")]
pub async fn refresh(req: Json<RefreshRequest>, pool: &State<DbPool>) -> Result<Value, ApiError> {
    let hashed_request_token = hash_refresh_token(&req.refresh_token);
    let user = sqlx::query!(
        r#"
        SELECT id, display_name, refresh_token_expires_at 
        FROM users 
        WHERE refresh_token_hash = $1
        "#,
        hashed_request_token
    )
    .fetch_optional(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Database error while validating token"))?;

    let user = match user {
        Some(u) => u,
        None => {
            return Err(ApiError::unauthorized(
                "Refresh token is invalid or revoked",
            ))
        }
    };

    if let Some(expires_at) = user.refresh_token_expires_at {
        if Utc::now() > expires_at {
            return Err(ApiError::unauthorized(
                "Refresh token has expired, please login again",
            ));
        }
    } else {
        return Err(ApiError::unauthorized("Invalid session"));
    }

    let new_raw_refresh_token = generate_refresh_token();
    let new_hashed_refresh_token = hash_refresh_token(&new_raw_refresh_token);
    let new_refresh_expires_at = Utc::now() + Duration::days(7);

    sqlx::query!(
        "UPDATE users SET refresh_token_hash = $1, refresh_token_expires_at = $2 WHERE id = $3",
        new_hashed_refresh_token,
        new_refresh_expires_at,
        user.id
    )
    .execute(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to update session"))?;

    let new_access_token = generate_access_token(user.id)
        .map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "Token refreshed successfully",
        "data": AuthResponse {
            access_token: new_access_token,
            refresh_token: new_raw_refresh_token,
            user_id: user.id,
            display_name: user.display_name,
        }
    }));
}

pub fn routes() -> Vec<rocket::Route> {
    return routes![register, login, refresh];
}
