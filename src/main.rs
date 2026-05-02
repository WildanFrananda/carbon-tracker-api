#[macro_use]
extern crate rocket;

use deadpool_redis::{Config, Runtime};
use dotenvy::dotenv;
use rocket::http::Status;
use rocket::serde::json::{json, Value};
use rocket::{get, launch, routes, Build, Rocket, State};
use sqlx::postgres::PgPoolOptions;
use sqlx::Error;
use sqlx::PgPool;
use std::env;

use crate::utils::error::ApiError;
use crate::utils::rate_limiter::RateLimitFairing;

mod api;
mod engine;
mod models;
mod services;
mod utils;

pub struct DbPool(PgPool);
pub struct RedisPool(deadpool_redis::Pool);

#[get("/")]
fn index() -> Value {
    return json!({
        "status": "success",
        "message": "Welcome to Personal Footprint Tracker API",
        "version": "0.1.0"
    });
}

#[get("/health")]
async fn health_check(pool: &State<DbPool>) -> Value {
    return match sqlx::query!("SELECT 1 AS is_alive")
        .fetch_one(&pool.0)
        .await
    {
        Ok(_) => json!({ "status": "success", "message": "Database connection is healty" }),
        Err(_) => json!({ "status": "error", "message": "Failed to connect database" }),
    };
}

async fn init_db() -> Result<PgPool, Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://tracker_user:tracker_pass@localhost:5432/carbon_tracker_db".to_string()
    });

    return PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await;
}

#[get("/429")]
fn to_many_request() -> Result<Value, ApiError> {
    Err(ApiError {
        status: Status::TooManyRequests,
        message: "Global rate limit exceeded. please slow down".into()
    })
}

#[get("/400")]
fn device_id_err() -> Result<Value, ApiError> {
    Err(ApiError::bad_request("X-Device-ID header is required"))
}

#[catch(400)]
fn bad_request() -> Value {
    json!({
        "status": "error",
        "message": "Invalid request. Please check your data format and parameters."
    })
}

#[catch(404)]
fn not_found() -> Value {
    json!({
        "status": "error",
        "message": "Resource not found."
    })
}

#[catch(422)]
fn unprocessable_entity() -> Value {
    json!({
        "status": "error",
        "message": "Data format is invalid or missing required fields. Make sure you provided all necessary fields correctly (e.g., correct category 'transport', 'food', 'energy', 'shopping')."
    })
}

#[catch(500)]
fn internal_error() -> Value {
    json!({
        "status": "error",
        "message": "Internal server error."
    })
}

pub async fn build_rocket() -> Rocket<Build> {
    dotenv().ok();

    let pool = init_db().await.expect("Failed to initialize database pool");

    let cfg = Config::from_url(env::var("REDIS_URL").unwrap());
    let redis_pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

    rocket::build()
        .manage(DbPool(pool))
        .manage(RedisPool(redis_pool))
        .attach(RateLimitFairing)
        .mount("/", routes![index, health_check])
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

#[launch]
async fn rocket() -> _ {
    build_rocket().await
}

#[cfg(test)]
mod tests {
    use super::build_rocket;
    use dotenvy::dotenv;
    use rocket::http::Status;
    use rocket::local::asynchronous::{Client, LocalResponse};
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn setup() {
        INIT.call_once(|| {
            dotenv().ok();
        });
    }

    #[tokio::test]
    async fn test_index_route() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response: LocalResponse = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body_str: String = response
            .into_string()
            .await
            .expect("Response body should be a string");
        assert!(body_str.contains("Welcome to Personal Footprint Tracker API"));
    }

    #[rocket::async_test]
    async fn test_health_check() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response = client.get("/health").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_unauthorized_access() {
        setup();
        let client = Client::tracked(build_rocket().await)
            .await
            .expect("Valid rocket instance");

        let response = client
            .get("/api/dashboard/daily?date=2026-04-17")
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
    }
}
