use crate::utils::error::ApiError;
use crate::RedisPool;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};

pub struct RateLimit;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RateLimit {
    type Error = ApiError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = req
            .client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".into());
        let key = format!("ratelimit:{}", ip);

        let redis_pool = match req.rocket().state::<RedisPool>() {
            Some(pool) => pool,
            None => {
                return Outcome::Error((
                    Status::InternalServerError,
                    ApiError::internal("Redis pool missing"),
                ))
            }
        };

        let mut conn = match redis_pool.0.get().await {
            Ok(c) => c,
            Err(_) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    ApiError::internal("Redis connection failed"),
                ))
            }
        };

        let limit = 100;
        let window = 60;

        let count: i64 = redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut *conn)
            .await
            .unwrap_or(0);

        if count == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(&key)
                .arg(window)
                .query_async(&mut *conn)
                .await
                .unwrap_or(());
        }

        if count > limit {
            return Outcome::Error((
                Status::TooManyRequests,
                ApiError {
                    status: Status::TooManyRequests,
                    message: "Too many requests. Please slow down bestie. fr.".into(),
                },
            ));
        }
        return Outcome::Success(RateLimit);
    }
}
