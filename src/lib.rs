#[macro_use]
extern crate rocket;

use deadpool_redis::{Config, Runtime};
use dotenvy::dotenv;
use rocket::http::Status;
use rocket::serde::json::{json, Value};
use rocket::{get, launch, routes, Build, Rocket, State, catch};
use sqlx::postgres::PgPoolOptions;
use sqlx::Error;
use sqlx::PgPool;
use std::env;

use crate::utils::error::ApiError;
use crate::utils::rate_limiter::RateLimitFairing;

pub mod api;
pub mod engine;
pub mod models;
pub mod services;
pub mod utils;
pub mod db; // Make sure db is exposed if it exists

pub struct DbPool(pub PgPool);
pub struct RedisPool(pub deadpool_redis::Pool);

#[get("/")]
pub fn index() -> Value {
    return json!({
        "status": "success",
        "message": "Welcome to Personal Footprint Tracker API",
        "version": "0.1.0"
    });
}

#[get("/health")]
pub async fn health_check(pool: &State<DbPool>) -> Value {
    return match sqlx::query!("SELECT 1 AS is_alive")
        .fetch_one(&pool.0)
        .await
    {
        Ok(_) => json!({ "status": "success", "message": "Database connection is healty" }),
        Err(_) => json!({ "status": "error", "message": "Failed to connect database" }),
    };
}

pub async fn init_db() -> Result<PgPool, Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://tracker_user:tracker_pass@localhost:5432/carbon_tracker_db".to_string()
    });

    return PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await;
}

#[get("/429")]
pub fn to_many_request() -> Result<Value, ApiError> {
    Err(ApiError {
        status: Status::TooManyRequests,
        message: "Global rate limit exceeded. please slow down".into()
    })
}

#[get("/400")]
pub fn device_id_err() -> Result<Value, ApiError> {
    Err(ApiError::bad_request("X-Device-ID header is required"))
}

#[catch(400)]
pub fn bad_request() -> Value {
    json!({
        "status": "error",
        "message": "Invalid request. Please check your data format and parameters."
    })
}

#[catch(404)]
pub fn not_found() -> Value {
    json!({
        "status": "error",
        "message": "Resource not found."
    })
}

#[catch(422)]
pub fn unprocessable_entity() -> Value {
    json!({
        "status": "error",
        "message": "Data format is invalid or missing required fields. Make sure you provided all necessary fields correctly (e.g., correct category 'transport', 'food', 'energy', 'shopping')."
    })
}

#[catch(500)]
pub fn internal_error() -> Value {
    json!({
        "status": "error",
        "message": "Internal server error."
    })
}

#[get("/.well-known/assetlinks.json")]
pub fn asset_links() -> rocket::serde::json::Value {
    rocket::serde::json::json!([{
        "relation": ["delegate_permission/common.handle_all_urls"],
        "target": {
            "namespace": "android_app",
            "package_name": "com.example.carbontracker",
            "sha256_cert_fingerprints": [
                "MASUKKAN_SHA256_CERTIFICATE_ANDA_DI_SINI"
            ]
        }
    }])
}

pub async fn build_rocket() -> Rocket<Build> {
    dotenv().ok();

    let pool = init_db().await.expect("Failed to initialize database pool");

    let cfg = Config::from_url(env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()));
    let redis_pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

    rocket::build()
        .manage(DbPool(pool))
        .manage(RedisPool(redis_pool))
        .attach(RateLimitFairing)
        .mount("/", routes![index, health_check, asset_links])
        .mount("/errors", routes![to_many_request, device_id_err])
        .mount("/api/auth", api::auth::routes())
        .mount("/api/activities", api::activities::routes())
        .mount("/api/dashboard", api::dashboard::routes())
        .mount("/api/insights", api::insights::routes())
        .mount("/api/gamification", api::gamification::routes())
        .mount("/api/user", api::user::routes())
        .register(
            "/",
            rocket::catchers![bad_request, not_found, unprocessable_entity, internal_error],
        )
}
