use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest};
use crate::utils::error::ApiError;
use crate::utils::jwt::generate_token;
use crate::DbPool;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
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

    let user = sqlx::query!(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id, display_name",
        req.email,
        password_hash,
        req.display_name
    )
    .fetch_one(&pool.0)
    .await
    .map_err(|_| ApiError::internal("Failed to register user"))?;

    let token =
        generate_token(user.id).map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "User registered successfully",
        "data": AuthResponse {
            token,
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

    let token =
        generate_token(user.id).map_err(|_| ApiError::internal("Failed to generate token"))?;

    return Ok(json!({
        "status": "success",
        "message": "Login successfully",
        "data": AuthResponse {
            token,
            user_id: user.id,
            display_name: user.display_name,
        }
    }));
}

pub fn routes() -> Vec<rocket::Route> {
    return routes![register, login];
}
