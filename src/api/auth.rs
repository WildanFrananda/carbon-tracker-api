use crate::models::user::{AuthResponse, ForgotPasswordRequest, GoogleLoginRequest, LoginRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest};
use crate::utils::error::ApiError;
use crate::utils::google::verify_google_token;
use crate::utils::jwt::{generate_access_token, generate_refresh_token, hash_refresh_token};
use crate::{DbPool, RedisPool};
use crate::services::email::send_reset_email;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use rocket::serde::json::{json, Json, Value};
use rocket::State;

#[post("/google", data = "<req>")]
pub async fn google_login(
    req: Json<GoogleLoginRequest>,
    pool: &State<DbPool>
) -> Result<Value, ApiError> {
    let google_info = verify_google_token(&req.id_token).await?;

    let user = sqlx::query!(
        "SELECT id, display_name, google_id FROM users WHERE google_id = $1 OR email = $2",
        google_info.sub, google_info.email
    )
    .fetch_optional(&pool.0).await
    .map_err(|_| ApiError::internal("Database error"))?;

    let user_id;
    let display_name;

    match user {
        Some(u) => {
            user_id = u.id;
            display_name = u.display_name;
            if u.google_id.is_none() {
                sqlx::query!("UPDATE users SET google_id = $1 WHERE id = $2", google_info.sub, u.id)
                    .execute(&pool.0).await.ok();
            }
        }
        None => {
            let new_user = sqlx::query!(
                "INSERT INTO users (email, display_name, google_id) VALUES ($1, $2, $3) RETURNING id, display_name",
                google_info.email, google_info.name, google_info.sub
            )
            .fetch_one(&pool.0).await
            .map_err(|_| ApiError::internal("Failed to creaate new user"))?;

            user_id = new_user.id;
            display_name = new_user.display_name;   
        }
    }

    let raw_refresh_token = generate_refresh_token();
    let hashed_refresh_token = hash_refresh_token(&raw_refresh_token);
    let refresh_expires_at = Utc::now() + Duration::days(7);

    sqlx::query!(
        "UPDATE users SET refresh_token_hash = $1, refresh_token_expires_at = $2 WHERE id = $3",
        hashed_refresh_token,
        refresh_expires_at,
        user_id
    )
    .execute(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to save login session"))?;

    let access_token = generate_access_token(user_id)
        .map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "Login successfully",
        "data": AuthResponse {
            access_token,
            refresh_token: raw_refresh_token,
            user_id,
            display_name,
        }
    }));
}

#[post("/forgot-password", data = "<req>")]
pub async fn forgot_password(req: Json<ForgotPasswordRequest>, pool: &State<DbPool>, redis: &State<RedisPool>) -> Result<Value, ApiError> {
    // .fetch_optional mengembalikan Result<Option<_>>, kita unwrap dulu lalu cek apakah Some
    let user_exists = sqlx::query!("SELECT id FROM users WHERE email = $1", req.email)
        .fetch_optional(&pool.0)
        .await
        .map_err(|_| ApiError::internal("Database error"))?
        .is_some();
    if user_exists {
        let token = uuid::Uuid::new_v4().to_string();
        let mut conn = redis.0.get().await.map_err(|_| ApiError::internal("Redis error"))?;
        
        let key = format!("password_reset:{}", token);
        let _: () = redis::cmd("SETEX").arg(&key).arg(3600).arg(&req.email).query_async(&mut *conn).await
            .map_err(|_| ApiError::internal("Gagal menyimpan token"))?;
        let email = req.email.clone();
        tokio::spawn(async move {
            let _ = send_reset_email(&email, &token);
        });
    }
    // Selalu return sukses untuk alasan keamanan (mencegah user enumeration)
    return Ok(json!({ "status": "success", "message": "Please check your email" }));
}

#[post("/reset-password", data = "<req>")]
pub async fn reset_password(req: Json<ResetPasswordRequest>, pool: &State<DbPool>, redis: &State<RedisPool>) -> Result<Value, ApiError> {
    let mut conn = redis.0.get().await.map_err(|_| ApiError::internal("Redis error"))?;
    let key = format!("password_reset:{}", req.token);
    // Ambil email dari Redis menggunakan token
    let email: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut *conn).await
        .map_err(|_| ApiError::internal("Gagal memverifikasi token"))?;
    let email = match email {
        Some(e) => e,
        None => return Err(ApiError::bad_request("Token tidak valid atau sudah kadaluarsa")),
    };
    // Hash password baru
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default().hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|_| ApiError::internal("Failed to hashing password"))?.to_string();
    // Update password di DB
    sqlx::query!("UPDATE users SET password_hash = $1 WHERE email = $2", password_hash, email)
        .execute(&pool.0).await
        .map_err(|_| ApiError::internal("Failed to update password"))?;
    // Hapus token dari Redis
    let _: () = redis::cmd("DEL").arg(&key).query_async(&mut *conn).await.ok().unwrap_or(());
    return Ok(json!({ "status": "success", "message": "Password reset successfully" }));
}

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

    let hash_str = match &user.password_hash {
        Some(h) => h.clone(),
        None => return Err(ApiError::unauthorized("Akun ini terdaftar via Google. Silakan login dengan Google.")),
    };
    let parsed_hash = PasswordHash::new(&hash_str)
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
    return routes![register, login, refresh, google_login, forgot_password, reset_password];
}
